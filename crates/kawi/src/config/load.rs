use std::path::Path;

use toml_edit::{Document, Item, Value};

use crate::options::{self, ConfigSection, OptionInput, OptionSpec};

use super::{statusline, Config, ConfigLoad, ConfigWarning};

pub(super) fn load_document(
    text: &str,
    document: &Document<&str>,
    theme_dir: Option<&Path>,
) -> ConfigLoad {
    let mut loader = Loader {
        text,
        theme_dir,
        config: Config::default(),
        warnings: Vec::new(),
    };
    for (key, item) in document.iter() {
        loader.load_top_level(key, item);
    }
    ConfigLoad {
        config: loader.config,
        warnings: loader.warnings,
    }
}

pub(super) struct Loader<'a> {
    text: &'a str,
    theme_dir: Option<&'a Path>,
    pub(super) config: Config,
    warnings: Vec<ConfigWarning>,
}

impl Loader<'_> {
    fn load_top_level(&mut self, key: &str, item: &Item) {
        if key == "history" {
            self.load_history(item);
            return;
        }
        let section = match key {
            "ui" => ConfigSection::Ui,
            "editor" => ConfigSection::Editor,
            "explorer" => ConfigSection::Explorer,
            _ => {
                self.unknown(key, item);
                return;
            }
        };
        self.load_section(section, key, item);
    }

    fn load_history(&mut self, item: &Item) {
        if item.is_table() {
            self.load_section(ConfigSection::History, "history", item);
            return;
        }
        let spec = options::lookup("history").expect("history is registered");
        self.load_option(spec, "history", item);
    }

    fn load_section(&mut self, section: ConfigSection, name: &str, item: &Item) {
        let Some(table) = item.as_table() else {
            self.warn(
                item.span(),
                name,
                item.to_string(),
                "section ignored, expected a table",
            );
            return;
        };
        for (key, item) in table.iter() {
            if section == ConfigSection::History && key == "undo_exclude" {
                self.load_undo_exclude(item);
                continue;
            }
            if section == ConfigSection::Ui && key == "statusline" {
                statusline::load(self, item);
                continue;
            }
            if section == ConfigSection::Explorer
                && matches!(key, "metadata" | "directory_metadata" | "dmd")
            {
                self.load_metadata(item);
                continue;
            }
            let Some(spec) = spec_for(section, key) else {
                self.unknown(&format!("{name}.{key}"), item);
                continue;
            };
            let warning_key = format!("{name}.{key}");
            self.load_option(spec, &warning_key, item);
        }
    }

    fn load_undo_exclude(&mut self, item: &Item) {
        let Some(array) = item.as_array() else {
            self.warn(
                item.span(),
                "history.undo_exclude",
                item.to_string(),
                "default exclusions",
            );
            return;
        };
        let mut values = Vec::with_capacity(array.len());
        for value in array.iter() {
            let Some(text) = value.as_str() else {
                self.warn(
                    value.span(),
                    "history.undo_exclude",
                    value.to_string(),
                    "default exclusions",
                );
                return;
            };
            values.push(text.to_string());
        }
        self.config.undo_exclude = Some(values);
    }

    fn load_metadata(&mut self, item: &Item) {
        let Some(spec) = options::lookup("directory_metadata") else {
            return;
        };
        let Some(value) = metadata_value(item) else {
            let fallback = spec.default_value().display();
            self.warn(
                item.span(),
                "explorer.metadata",
                item.to_string(),
                format!("{fallback}, expected a list or string"),
            );
            self.config.values.insert(spec.name, spec.default_value());
            return;
        };
        self.load_option_input(
            spec,
            "explorer.metadata",
            options::OptionInput::Text(&value),
            item.span(),
            item.to_string(),
        );
    }

    fn load_option(&mut self, spec: &'static OptionSpec, key: &str, item: &Item) {
        let Some(input) = option_input(item) else {
            let fallback = spec.default_value().display();
            self.warn(
                item.span(),
                key,
                item.to_string(),
                format!("{}, expected {}", fallback, expected_type(spec)),
            );
            self.config.values.insert(spec.name, spec.default_value());
            return;
        };
        self.load_option_input(spec, key, input, item.span(), item.to_string());
    }

    fn load_option_input(
        &mut self,
        spec: &'static OptionSpec,
        key: &str,
        input: options::OptionInput<'_>,
        span: Option<std::ops::Range<usize>>,
        value: String,
    ) {
        let fallback = spec.default_value().display();
        let context = options::OptionContext {
            theme_dir: self.theme_dir,
        };
        match options::validate_spec(spec, input, &context) {
            Ok(value) => {
                self.config.values.insert(spec.name, value);
            }
            Err(_) => {
                self.config.values.insert(spec.name, spec.default_value());
                self.warn(span, key, value, fallback);
            }
        }
    }

    fn unknown(&mut self, key: &str, item: &Item) {
        self.warn(item.span(), key, item.to_string(), "ignored");
    }

    pub(super) fn warn(
        &mut self,
        span: Option<std::ops::Range<usize>>,
        key: impl Into<String>,
        value: impl Into<String>,
        fallback: impl Into<String>,
    ) {
        self.warnings.push(ConfigWarning::new(
            line_at(self.text, span),
            key,
            value,
            fallback,
        ));
    }
}

fn metadata_value(item: &Item) -> Option<String> {
    if let Some(array) = item.as_array() {
        if array.is_empty() {
            return Some("none".to_string());
        }
        let mut values = Vec::with_capacity(array.len());
        for value in array.iter() {
            values.push(value.as_str()?.to_string());
        }
        return Some(values.join(","));
    }
    item.as_str().map(str::to_string)
}

fn spec_for(section: ConfigSection, key: &str) -> Option<&'static OptionSpec> {
    options::OPTIONS.iter().find(|spec| {
        spec.section == section
            && (spec.config_key == key || spec.name == key || spec.aliases.contains(&key))
    })
}

fn option_input(item: &Item) -> Option<OptionInput<'_>> {
    match item.as_value()? {
        Value::Boolean(value) => Some(OptionInput::Bool(*value.value())),
        Value::Integer(value) => Some(OptionInput::Integer(*value.value())),
        Value::String(value) => Some(OptionInput::Text(value.value())),
        _ => None,
    }
}

fn expected_type(spec: &OptionSpec) -> &'static str {
    match spec.kind {
        options::OptionKind::Bool => "a boolean",
        options::OptionKind::Int { .. } => "an integer",
        options::OptionKind::Enum(_) => "a name",
        options::OptionKind::List(_) => "a list or comma-separated names",
        options::OptionKind::Text => "text",
    }
}

pub(crate) fn line_at(text: &str, span: Option<std::ops::Range<usize>>) -> Option<usize> {
    span.map(|span| {
        text[..span.start]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 1
    })
}
