use std::collections::{BTreeMap, HashSet};
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::protocol::PROTOCOL_VERSION;

const MANIFEST_FILE: &str = "plugin.toml";
const DEFAULT_TIMEOUT_MS: u64 = 5_000;
const MIN_TIMEOUT_MS: u64 = 100;
const MAX_TIMEOUT_MS: u64 = 60_000;
const MAX_NAME_LEN: usize = 64;
const PLUGIN_DIR_TOKEN: &str = "${plugin_dir}";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub protocol: u64,
    #[serde(default)]
    pub description: String,
    pub command: Vec<String>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub commands: Vec<PluginCommand>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginCommand {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub keybindings: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub dir: PathBuf,
    pub path: PathBuf,
}

#[derive(Debug, Default)]
pub struct PluginCatalog {
    pub plugins: Vec<LoadedPlugin>,
    pub warnings: Vec<PluginWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginWarning {
    pub path: PathBuf,
    pub message: String,
}

impl fmt::Display for PluginWarning {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path.display(), self.message)
    }
}

impl LoadedPlugin {
    pub fn command_name(&self, command: &PluginCommand) -> String {
        format!("{}.{}", self.manifest.name, command.id)
    }

    pub fn resolved_command(&self) -> Vec<OsString> {
        self.manifest
            .command
            .iter()
            .map(|argument| {
                OsString::from(argument.replace(PLUGIN_DIR_TOKEN, &self.dir.to_string_lossy()))
            })
            .collect()
    }
}

pub fn load_catalog(dir: &Path) -> PluginCatalog {
    let mut catalog = PluginCatalog::default();
    let entries = match read_entries(dir) {
        Ok(entries) => entries,
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                catalog.warnings.push(PluginWarning {
                    path: dir.to_path_buf(),
                    message: format!("cannot read plugin directory: {error}"),
                });
            }
            return catalog;
        }
    };
    let mut names = HashSet::new();
    for entry in entries {
        let Some(plugin) = load_plugin(&entry, &mut catalog.warnings) else {
            continue;
        };
        if !names.insert(plugin.manifest.name.clone()) {
            catalog.warnings.push(PluginWarning {
                path: plugin.path.clone(),
                message: format!("duplicate plugin name: {}", plugin.manifest.name),
            });
            continue;
        }
        catalog.plugins.push(plugin);
    }
    catalog
}

fn read_entries(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut entries = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    entries.sort();
    Ok(entries)
}

fn load_plugin(dir: &Path, warnings: &mut Vec<PluginWarning>) -> Option<LoadedPlugin> {
    let path = dir.join(MANIFEST_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                warnings.push(PluginWarning {
                    path: path.clone(),
                    message: format!("cannot read manifest: {error}"),
                });
            }
            return None;
        }
    };
    let manifest = match toml::from_str::<PluginManifest>(&text) {
        Ok(manifest) => manifest,
        Err(error) => {
            warnings.push(PluginWarning {
                path: path.clone(),
                message: error.to_string(),
            });
            return None;
        }
    };
    match validate_manifest(&manifest) {
        Ok(()) => Some(LoadedPlugin {
            manifest,
            dir: dir.to_path_buf(),
            path,
        }),
        Err(message) => {
            warnings.push(PluginWarning {
                path: path.clone(),
                message,
            });
            None
        }
    }
}

fn validate_manifest(manifest: &PluginManifest) -> Result<(), String> {
    validate_name(&manifest.name)?;
    if manifest.version.trim().is_empty() {
        return Err("version must not be empty".to_string());
    }
    if manifest.protocol != PROTOCOL_VERSION {
        return Err(format!(
            "unsupported protocol {}, expected {}",
            manifest.protocol, PROTOCOL_VERSION
        ));
    }
    validate_command(&manifest.command)?;
    validate_timeout(manifest.timeout_ms)?;
    validate_commands(&manifest.commands)
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > MAX_NAME_LEN {
        return Err(format!("name must be 1 to {MAX_NAME_LEN} characters"));
    }
    if !name.chars().all(|character| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '-' | '_')
    }) {
        return Err(
            "name may contain only lowercase letters, digits, hyphens, and underscores".to_string(),
        );
    }
    if !name
        .starts_with(|character: char| character.is_ascii_lowercase() || character.is_ascii_digit())
    {
        return Err("name must start with a letter or digit".to_string());
    }
    Ok(())
}

fn validate_command(command: &[String]) -> Result<(), String> {
    if command.is_empty() {
        return Err("command must not be empty".to_string());
    }
    if command.iter().any(|argument| argument.is_empty()) {
        return Err("command arguments must not be empty".to_string());
    }
    Ok(())
}

fn validate_timeout(timeout_ms: u64) -> Result<(), String> {
    if !(MIN_TIMEOUT_MS..=MAX_TIMEOUT_MS).contains(&timeout_ms) {
        return Err(format!(
            "timeout_ms must be between {MIN_TIMEOUT_MS} and {MAX_TIMEOUT_MS}"
        ));
    }
    Ok(())
}

fn validate_commands(commands: &[PluginCommand]) -> Result<(), String> {
    let mut ids = HashSet::new();
    for command in commands {
        validate_command_id(&command.id)?;
        if command.title.trim().is_empty() {
            return Err(format!("command {} has an empty title", command.id));
        }
        if !ids.insert(command.id.as_str()) {
            return Err(format!("duplicate command id: {}", command.id));
        }
        validate_keybindings(&command.keybindings)?;
    }
    Ok(())
}

fn validate_command_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > MAX_NAME_LEN {
        return Err(format!("command id must be 1 to {MAX_NAME_LEN} characters"));
    }
    if !id.chars().all(|character| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '.' | '-' | '_')
    }) {
        return Err(format!("command id {id} contains an invalid character"));
    }
    if !id
        .starts_with(|character: char| character.is_ascii_lowercase() || character.is_ascii_digit())
    {
        return Err(format!("command id {id} must start with a letter or digit"));
    }
    Ok(())
}

fn validate_keybindings(bindings: &BTreeMap<String, Vec<String>>) -> Result<(), String> {
    for (mode, sequences) in bindings {
        if !matches!(mode.as_str(), "normal" | "insert" | "visual") {
            return Err(format!("unknown keybinding mode: {mode}"));
        }
        if sequences.is_empty() {
            return Err(format!("keybinding mode {mode} must not be empty"));
        }
        if sequences.iter().any(|sequence| sequence.trim().is_empty()) {
            return Err(format!("keybinding mode {mode} contains an empty sequence"));
        }
    }
    Ok(())
}

fn default_timeout_ms() -> u64 {
    DEFAULT_TIMEOUT_MS
}

#[cfg(test)]
mod tests;
