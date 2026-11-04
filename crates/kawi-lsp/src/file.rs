use crate::editing;
use crate::editing::token_is_valid;
use crate::{FormatterConfig, LanguageConfig, LanguageWarning, ServerConfig};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use toml::Value;

mod indent;

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
#[serde(default, deny_unknown_fields)]
pub(crate) struct FormatterOverride {
    command: Option<String>,
    args: Option<Vec<String>>,
    env: Option<BTreeMap<String, String>>,
    stdin: Option<bool>,
}

impl FormatterOverride {
    fn overlay(self, mut formatter: FormatterConfig) -> FormatterConfig {
        if let Some(command) = self.command {
            formatter.command = command;
        }
        if let Some(args) = self.args {
            formatter.args = args;
        }
        if let Some(env) = self.env {
            formatter.env = env;
        }
        if let Some(stdin) = self.stdin {
            formatter.stdin = stdin;
        }
        formatter
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
    formatter: Option<Value>,
    shiftwidth: Option<Value>,
    expandtab: Option<Value>,
    tabstop: Option<Value>,
    indent_after_suffixes: Option<Value>,
    indent_after_keywords: Option<Value>,
    indent_brackets: Option<Value>,
    line_comment: Option<Value>,
    block_comment: Option<Value>,
    auto_pairs: Option<Value>,
    match_pairs: Option<Value>,
    enabled: Option<bool>,
}

fn apply_formatter(
    language: &mut LanguageConfig,
    value: Value,
    warnings: &mut Vec<LanguageWarning>,
) {
    if matches!(&value, Value::Boolean(false)) {
        language.formatter = None;
        return;
    }
    if !matches!(&value, Value::Table(_)) {
        warnings.push(LanguageWarning::new(format!(
            "language {}: formatter must be a table or false, got {value}; keeping the previous formatter",
            language.name
        )));
        return;
    }
    let previous = language.formatter.clone();
    match value.try_into::<FormatterOverride>() {
        Ok(override_config) => {
            let formatter = override_config.overlay(previous.unwrap_or_default());
            if formatter.command.trim().is_empty() {
                warnings.push(LanguageWarning::new(format!(
                    "language {}: formatter command must be a non-empty string; keeping the previous formatter",
                    language.name
                )));
            } else {
                language.formatter = Some(formatter);
            }
        }
        Err(error) => warnings.push(LanguageWarning::new(format!(
            "language {}: invalid formatter: {}; keeping the previous formatter",
            language.name,
            error.message()
        ))),
    }
}

fn parse_string_list(
    language: &str,
    value: Value,
    field: &str,
    warnings: &mut Vec<LanguageWarning>,
) -> Option<Vec<String>> {
    let display = value.to_string();
    let parsed = match value {
        Value::Array(tokens) => tokens
            .iter()
            .map(|token| match token {
                Value::String(text) if token_is_valid(text) => Some(text.clone()),
                _ => None,
            })
            .collect::<Option<Vec<String>>>(),
        _ => None,
    };
    if parsed.is_none() {
        warnings.push(LanguageWarning::new(format!(
            "language {language}: {field} must be an array of non-empty strings, got {display}; keeping the previous rules"
        )));
    }
    parsed
}

fn apply_indent_brackets(
    language: &mut LanguageConfig,
    value: Value,
    warnings: &mut Vec<LanguageWarning>,
) {
    match value {
        Value::Boolean(value) => language.indent_brackets = value,
        value => warnings.push(LanguageWarning::new(format!(
            "language {}: indent_brackets must be a boolean, got {value}; keeping the previous rule",
            language.name
        ))),
    }
}

impl LanguageOverride {
    pub(crate) fn overlay(
        self,
        mut language: LanguageConfig,
    ) -> (LanguageConfig, Vec<LanguageWarning>) {
        let mut warnings = Vec::new();
        indent::apply(
            &mut language.indent_override,
            indent::IndentOverride {
                shiftwidth: self.shiftwidth,
                expandtab: self.expandtab,
                tabstop: self.tabstop,
            },
            &language.name,
            &mut warnings,
        );
        if let Some(value) = self.indent_after_suffixes {
            if let Some(tokens) = parse_string_list(
                &language.name,
                value,
                "indent_after_suffixes",
                &mut warnings,
            ) {
                language.indent_after_suffixes = tokens;
            }
        }
        if let Some(value) = self.indent_after_keywords {
            if let Some(tokens) = parse_string_list(
                &language.name,
                value,
                "indent_after_keywords",
                &mut warnings,
            ) {
                language.indent_after_keywords = tokens;
            }
        }
        if let Some(value) = self.indent_brackets {
            apply_indent_brackets(&mut language, value, &mut warnings);
        }
        if let Some(value) = self.line_comment {
            editing::apply_line_comment(&mut language, value, &mut warnings);
        }
        if let Some(value) = self.block_comment {
            editing::apply_block_comment(&mut language, value, &mut warnings);
        }
        if let Some(value) = self.auto_pairs {
            editing::apply_auto_pairs(&mut language, value, &mut warnings);
        }
        if let Some(value) = self.match_pairs {
            editing::apply_match_pairs(&mut language, value, &mut warnings);
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
        if let Some(value) = self.formatter {
            apply_formatter(&mut language, value, &mut warnings);
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
