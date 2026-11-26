use std::cmp::Ordering;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use crate::abi;
use crate::error::GrammarError;
use crate::file;
use crate::hash;
use crate::limits;
use crate::version;

pub const PACK_FORMAT: u32 = 1;
pub const MANIFEST_FILE: &str = "manifest.toml";

const MAX_ID_LEN: usize = 64;
const MAX_REPOSITORY_LEN: usize = 512;
const MAX_LICENSE_LEN: usize = 128;
const MAX_REVISION_LEN: usize = 128;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format: u32,
    pub id: String,
    pub version: String,
    pub language: String,
    pub grammar: PathBuf,
    pub grammar_sha256: String,
    pub grammar_revision: String,
    pub abi_min: usize,
    pub abi_max: usize,
    pub minimum_kawi: String,
    pub repository: String,
    pub license: String,
    pub queries: Queries,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Queries {
    pub highlights: PathBuf,
    pub highlights_sha256: String,
    #[serde(default)]
    pub injections: Option<PathBuf>,
    #[serde(default)]
    pub injections_sha256: Option<String>,
    #[serde(default)]
    pub locals: Option<PathBuf>,
    #[serde(default)]
    pub locals_sha256: Option<String>,
}

impl Manifest {
    pub fn from_toml(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }

    pub fn load(path: &Path) -> Result<Self, GrammarError> {
        let metadata =
            fs::symlink_metadata(path).map_err(|source| GrammarError::io(path, source))?;
        if metadata.file_type().is_symlink() {
            return Err(GrammarError::Symlink(path.to_path_buf()));
        }
        let bytes = file::read_bounded(path, limits::MAX_MANIFEST_BYTES)?;
        let text = String::from_utf8(bytes).map_err(|source| GrammarError::InvalidUtf8 {
            path: path.to_path_buf(),
            source,
        })?;
        let manifest = Self::from_toml(&text).map_err(|source| GrammarError::Manifest {
            path: path.to_path_buf(),
            source,
        })?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), GrammarError> {
        if self.format != PACK_FORMAT {
            return Err(GrammarError::UnsupportedFormat {
                found: self.format,
                supported: PACK_FORMAT,
            });
        }
        validate_id("id", &self.id)?;
        validate_version("version", &self.version)?;
        validate_id("language", &self.language)?;
        validate_relative("grammar", &self.grammar)?;
        validate_hash("grammar_sha256", &self.grammar_sha256)?;
        validate_text("grammar_revision", &self.grammar_revision, MAX_REVISION_LEN)?;
        if self.abi_min == 0 {
            return Err(GrammarError::InvalidManifest(
                "abi_min must be greater than zero".to_string(),
            ));
        }
        if self.abi_min > self.abi_max {
            return Err(GrammarError::InvalidManifest(format!(
                "abi_min {} is greater than abi_max {}",
                self.abi_min, self.abi_max
            )));
        }
        validate_version("minimum_kawi", &self.minimum_kawi)?;
        validate_text("repository", &self.repository, MAX_REPOSITORY_LEN)?;
        validate_text("license", &self.license, MAX_LICENSE_LEN)?;
        self.queries.validate()
    }

    pub fn ensure_compatible(
        &self,
        host_version: &str,
        host_min: usize,
        host_max: usize,
    ) -> Result<(), GrammarError> {
        if version::compare(&self.minimum_kawi, host_version) == Some(Ordering::Greater) {
            return Err(GrammarError::UnsupportedKawi {
                required: self.minimum_kawi.clone(),
                current: host_version.to_string(),
            });
        }
        self.ensure_abi_range(host_min, host_max)
    }

    pub fn ensure_abi_range(&self, host_min: usize, host_max: usize) -> Result<(), GrammarError> {
        if self.abi_max < host_min || self.abi_min > host_max {
            return Err(GrammarError::AbiRange {
                pack_min: self.abi_min,
                pack_max: self.abi_max,
                host_min,
                host_max,
            });
        }
        Ok(())
    }

    pub fn ensure_abi(&self, abi: usize) -> Result<(), GrammarError> {
        if abi < self.abi_min || abi > self.abi_max {
            return Err(GrammarError::Abi {
                found: abi,
                min: self.abi_min,
                max: self.abi_max,
            });
        }
        Ok(())
    }

    pub fn ensure_host_abi(&self) -> Result<(), GrammarError> {
        self.ensure_abi_range(abi::MIN_ABI, abi::MAX_ABI)
    }

    pub fn ensure_host_compatible(&self, host_version: &str) -> Result<(), GrammarError> {
        self.ensure_compatible(host_version, abi::MIN_ABI, abi::MAX_ABI)
    }
}

impl Queries {
    fn validate(&self) -> Result<(), GrammarError> {
        validate_relative("queries.highlights", &self.highlights)?;
        validate_hash("queries.highlights_sha256", &self.highlights_sha256)?;
        validate_optional_query(
            "queries.injections",
            self.injections.as_deref(),
            self.injections_sha256.as_deref(),
        )?;
        validate_optional_query(
            "queries.locals",
            self.locals.as_deref(),
            self.locals_sha256.as_deref(),
        )
    }
}

fn validate_optional_query(
    name: &str,
    path: Option<&Path>,
    sha256: Option<&str>,
) -> Result<(), GrammarError> {
    match (path, sha256) {
        (None, None) => Ok(()),
        (Some(path), Some(sha256)) => {
            validate_relative(name, path)?;
            validate_hash(&format!("{name}_sha256"), sha256)
        }
        (Some(_), None) => Err(GrammarError::InvalidManifest(format!(
            "{name} requires {name}_sha256"
        ))),
        (None, Some(_)) => Err(GrammarError::InvalidManifest(format!(
            "{name}_sha256 requires {name}"
        ))),
    }
}

fn validate_id(name: &str, value: &str) -> Result<(), GrammarError> {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return Err(GrammarError::InvalidManifest(format!(
            "{name} must not be empty"
        )));
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return Err(GrammarError::InvalidManifest(format!(
            "{name} must start with a lowercase letter or digit"
        )));
    }
    if value.len() > MAX_ID_LEN
        || !characters.all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '_' | '-')
        })
    {
        return Err(GrammarError::InvalidManifest(format!(
            "{name} must use lowercase letters, digits, '-' or '_'"
        )));
    }
    Ok(())
}

fn validate_version(name: &str, value: &str) -> Result<(), GrammarError> {
    if version::Version::is_valid(value) {
        Ok(())
    } else {
        Err(GrammarError::InvalidManifest(format!(
            "{name} must be a semantic version"
        )))
    }
}

fn validate_relative(name: &str, value: &Path) -> Result<(), GrammarError> {
    if value.as_os_str().is_empty()
        || value.to_string_lossy().contains('\\')
        || !value
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(GrammarError::InvalidManifest(format!(
            "{name} must be a relative path inside the pack"
        )));
    }
    Ok(())
}

fn validate_hash(name: &str, value: &str) -> Result<(), GrammarError> {
    if hash::is_sha256_hex(value) {
        Ok(())
    } else {
        Err(GrammarError::InvalidManifest(format!(
            "{name} must be a 64 character sha256 hex digest"
        )))
    }
}

fn validate_text(name: &str, value: &str, max_len: usize) -> Result<(), GrammarError> {
    if value.is_empty() || value.len() > max_len || value.chars().any(char::is_control) {
        return Err(GrammarError::InvalidManifest(format!(
            "{name} must be non-empty, at most {max_len} bytes, and contain no control characters"
        )));
    }
    Ok(())
}
