use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;

use serde::Deserialize;

use crate::capability::{Capability, CapabilityRegistry, CapabilityRequests};

mod load;
mod runtime;
mod validation;

pub use load::load_catalog;
pub use runtime::{validate_component_path, PluginRuntime};

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
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub coalesce_queued: bool,
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

    pub fn command(&self, id: &str) -> Option<&PluginCommand> {
        self.manifest
            .commands
            .iter()
            .find(|command| command.id == id)
    }

    pub fn command_requirements(&self) -> BTreeMap<String, Vec<Capability>> {
        self.manifest
            .commands
            .iter()
            .map(|command| {
                let capabilities = command
                    .capabilities
                    .iter()
                    .filter_map(|name| Capability::parse(name))
                    .collect();
                (command.id.clone(), capabilities)
            })
            .collect()
    }

    pub fn denied_command(
        &self,
        registry: &CapabilityRegistry,
        command: &str,
    ) -> Option<Capability> {
        self.command(command)?
            .capabilities
            .iter()
            .filter_map(|name| Capability::parse(name))
            .find(|capability| !registry.has(&self.manifest.name, *capability))
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

pub fn validate_plugin_name(name: &str) -> Result<(), String> {
    validation::validate_name(name)
}

fn default_timeout_ms() -> u64 {
    DEFAULT_TIMEOUT_MS
}

#[cfg(test)]
mod tests;
