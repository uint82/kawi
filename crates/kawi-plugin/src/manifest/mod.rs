use std::collections::{BTreeMap, HashSet};
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::capability::{parse_capability_requests, CapabilityRequests};

mod runtime;
mod validation;

pub use runtime::PluginRuntime;

const MANIFEST_FILE: &str = "plugin.toml";
const DEFAULT_TIMEOUT_MS: u64 = 5_000;
const PLUGIN_DIR_TOKEN: &str = "${plugin_dir}";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub protocol: u64,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub runtime: PluginRuntime,
    #[serde(default)]
    pub component: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub command: Vec<String>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub commands: Vec<PluginCommand>,
    #[serde(default)]
    pub subscribes: Vec<String>,
    #[serde(default)]
    pub debounce: BTreeMap<String, u64>,
    #[serde(default)]
    pub activation_events: Vec<ActivationEvent>,
    #[serde(default)]
    pub capabilities: toml::Table,
    #[serde(skip)]
    pub capability_requests: CapabilityRequests,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivationEvent {
    pub event: String,
    #[serde(default)]
    pub filetypes: Vec<String>,
    #[serde(default)]
    pub globs: Vec<String>,
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
    pub computed_hash: Option<String>,
    pub disabled_reason: Option<String>,
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

    pub fn component_path(&self) -> Option<PathBuf> {
        self.manifest
            .component
            .as_ref()
            .map(|component| self.dir.join(component))
    }

    pub fn component_hash(&self) -> Option<&str> {
        self.computed_hash.as_deref()
    }

    pub fn declared_hash(&self) -> Option<&str> {
        self.manifest.sha256.as_deref()
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
    let mut manifest = match toml::from_str::<PluginManifest>(&text) {
        Ok(manifest) => manifest,
        Err(error) => {
            warnings.push(PluginWarning {
                path: path.clone(),
                message: error.to_string(),
            });
            return None;
        }
    };
    match validation::validate_manifest(&mut manifest, dir) {
        Ok(artifact) => {
            if let Some(reason) = &artifact.disabled_reason {
                warnings.push(PluginWarning {
                    path: path.clone(),
                    message: reason.clone(),
                });
            }
            let (mut requests, capability_warnings) =
                parse_capability_requests(&manifest.capabilities);
            if let Some(hash) = artifact.computed_hash.as_deref() {
                requests.set_artifact_hash(hash);
            }
            manifest.capability_requests = requests;
            for warning in capability_warnings {
                warnings.push(PluginWarning {
                    path: path.clone(),
                    message: format!("capabilities: {warning}"),
                });
            }
            Some(LoadedPlugin {
                manifest,
                dir: dir.to_path_buf(),
                path,
                computed_hash: artifact.computed_hash,
                disabled_reason: artifact.disabled_reason,
            })
        }
        Err(message) => {
            warnings.push(PluginWarning {
                path: path.clone(),
                message,
            });
            None
        }
    }
}

fn default_timeout_ms() -> u64 {
    DEFAULT_TIMEOUT_MS
}

#[cfg(test)]
mod tests;
