use crate::{LanguageConfig, LanguageWarning, ServerConfig, MAX_SHIFTWIDTH};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use toml::Value;

#[derive(Default, Deserialize)]
#[serde(default)]
pub(crate) struct LanguagesFile {
    pub(crate) server: BTreeMap<String, ServerOverride>,
    pub(crate) language: Vec<LanguageOverride>,
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub(crate) struct ServerOverride {
    command: Option<String>,
    args: Option<Vec<String>>,
    env: Option<BTreeMap<String, String>>,
    initialization_options: Option<Value>,
    settings: Option<Value>,
    enabled: Option<bool>,
}

impl ServerOverride {
    pub(crate) fn overlay(self, mut server: ServerConfig) -> ServerConfig {
        if let Some(command) = self.command {
            server.command = command;
        }
        if let Some(args) = self.args {
            server.args = args;
        }
        if let Some(env) = self.env {
            server.env = env;
        }
        if let Some(options) = self.initialization_options {
            server.initialization_options = Some(options);
        }
        if let Some(settings) = self.settings {
            server.settings = Some(settings);
        }
        if let Some(enabled) = self.enabled {
            server.enabled = enabled;
        }
        server
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub(crate) struct LanguageOverride {
    pub(crate) name: String,
    language_id: Option<String>,
    grammar: Option<String>,
    extensions: Option<Vec<String>>,
    filenames: Option<Vec<String>>,
    root_markers: Option<Vec<String>>,
    servers: Option<Vec<String>>,
    shiftwidth: Option<Value>,
    expandtab: Option<Value>,
    enabled: Option<bool>,
}

fn apply_shiftwidth(
    language: &mut LanguageConfig,
    value: Value,
    warnings: &mut Vec<LanguageWarning>,
) {
    let parsed = match value {
        Value::Integer(value) if (0..=MAX_SHIFTWIDTH as i64).contains(&value) => {
            Some(value as usize)
        }
        value => {
            warnings.push(LanguageWarning::new(format!(
                "language {}: shiftwidth must be an integer from 0 to {MAX_SHIFTWIDTH}, got {value}; using editor settings",
                language.name
            )));
            None
        }
    };
    language.shiftwidth = parsed;
}

fn apply_expandtab(
    language: &mut LanguageConfig,
    value: Value,
    warnings: &mut Vec<LanguageWarning>,
) {
    let parsed = match value {
        Value::Boolean(value) => Some(value),
        value => {
            warnings.push(LanguageWarning::new(format!(
                "language {}: expandtab must be a boolean, got {value}; using editor settings",
                language.name
            )));
            None
        }
    };
    language.expandtab = parsed;
}

impl LanguageOverride {
    pub(crate) fn overlay(
        self,
        mut language: LanguageConfig,
    ) -> (LanguageConfig, Vec<LanguageWarning>) {
        let mut warnings = Vec::new();
        if let Some(value) = self.shiftwidth {
            apply_shiftwidth(&mut language, value, &mut warnings);
        }
        if let Some(value) = self.expandtab {
            apply_expandtab(&mut language, value, &mut warnings);
        }
        if let Some(language_id) = self.language_id {
            language.language_id = language_id;
        }
        if let Some(grammar) = self.grammar {
            language.grammar = Some(grammar);
        }
        if let Some(extensions) = self.extensions {
            language.extensions = extensions;
        }
        if let Some(filenames) = self.filenames {
            language.filenames = filenames;
        }
        if let Some(markers) = self.root_markers {
            language.root_markers = markers;
        }
        if let Some(servers) = self.servers {
            language.servers = servers;
        }
        if let Some(enabled) = self.enabled {
            language.enabled = enabled;
        }
        (language, warnings)
    }

    pub(crate) fn into_config(self) -> (LanguageConfig, Vec<LanguageWarning>) {
        let name = self.name.clone();
        let language = LanguageConfig::new(name.clone(), name);
        self.overlay(language)
    }
}

pub(crate) fn load(path: &Path) -> (LanguagesFile, Vec<LanguageWarning>) {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return (LanguagesFile::default(), Vec::new());
        }
        Err(error) => {
            return (
                LanguagesFile::default(),
                vec![LanguageWarning::new(error.to_string())],
            )
        }
    };
    match toml::from_str(&text) {
        Ok(file) => (file, Vec::new()),
        Err(error) => {
            let line = error.span().map(|span| line_at(&text, span.start));
            (
                LanguagesFile::default(),
                vec![LanguageWarning::new(error.message()).at_line(line)],
            )
        }
    }
}

fn line_at(text: &str, offset: usize) -> usize {
    text.get(..offset)
        .map(|before| before.bytes().filter(|byte| *byte == b'\n').count() + 1)
        .unwrap_or(1)
}
