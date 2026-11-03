use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use expand::{expand_json, expand_path, expand_string};

mod expand;
mod parse;
#[cfg(test)]
mod tests;

const ADAPTER_FIELDS: &[&str] = &["command", "args", "env", "cwd"];
const CONFIGURATION_FIELDS: &[&str] = &["adapter", "languages", "request"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebugConfiguration {
    pub name: String,
    pub adapter: String,
    pub languages: Vec<String>,
    pub request: String,
    pub arguments: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedConfiguration {
    pub name: String,
    pub adapter: String,
    pub request: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cwd: Option<PathBuf>,
    pub arguments: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugContext {
    pub workspace: PathBuf,
    pub file: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugWarning {
    path: PathBuf,
    message: String,
}

impl DebugWarning {
    fn new(path: &Path, message: impl Into<String>) -> Self {
        Self {
            path: path.to_path_buf(),
            message: message.into(),
        }
    }

    pub fn format(&self) -> String {
        format!("{}: {}", self.path.display(), self.message)
    }
}

#[derive(Debug, Default)]
pub struct DebugConfig {
    adapters: BTreeMap<String, AdapterConfig>,
    configurations: BTreeMap<String, DebugConfiguration>,
    warnings: Vec<DebugWarning>,
}

impl DebugConfig {
    pub fn load(paths: &[PathBuf]) -> Self {
        let mut config = Self::default();
        for path in paths {
            config.load_file(path);
        }
        config
    }

    pub fn adapters(&self) -> impl Iterator<Item = (&str, &AdapterConfig)> {
        self.adapters
            .iter()
            .map(|(name, adapter)| (name.as_str(), adapter))
    }

    pub fn configurations(&self) -> impl Iterator<Item = &DebugConfiguration> {
        self.configurations.values()
    }

    pub fn configuration(&self, name: &str) -> Option<&DebugConfiguration> {
        self.configurations.get(name)
    }

    pub fn default_for(&self, language_id: Option<&str>) -> Option<&DebugConfiguration> {
        self.configurations
            .values()
            .find(|configuration| {
                language_id.is_some_and(|language_id| {
                    configuration
                        .languages
                        .iter()
                        .any(|language| language == language_id)
                })
            })
            .or_else(|| self.configurations.values().next())
    }

    pub fn warnings(&self) -> &[DebugWarning] {
        &self.warnings
    }

    pub fn expand(
        &self,
        name: &str,
        context: &DebugContext,
    ) -> Result<ExpandedConfiguration, String> {
        let configuration = self
            .configurations
            .get(name)
            .ok_or_else(|| format!("debug configuration {name} not found"))?;
        let adapter = self
            .adapters
            .get(&configuration.adapter)
            .ok_or_else(|| format!("debug adapter {} not found", configuration.adapter))?;
        let arguments = expand_json(configuration.arguments.clone(), context)?;
        let command = expand_string(&adapter.command, context)?;
        let args = adapter
            .args
            .iter()
            .map(|argument| expand_string(argument, context))
            .collect::<Result<Vec<_>, _>>()?;
        let env = adapter
            .env
            .iter()
            .map(|(key, value)| Ok((key.clone(), expand_string(value, context)?)))
            .collect::<Result<BTreeMap<_, _>, String>>()?;
        let cwd = adapter
            .cwd
            .as_ref()
            .map(|cwd| expand_path(cwd, context))
            .transpose()?;
        Ok(ExpandedConfiguration {
            name: configuration.name.clone(),
            adapter: configuration.adapter.clone(),
            request: configuration.request.clone(),
            command,
            args,
            env,
            cwd,
            arguments,
        })
    }
}
