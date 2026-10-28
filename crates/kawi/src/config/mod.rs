use crate::options::{self, OptionValue};
use anyhow::Result;
use kawi_tui::statusbar::StatusBarConfig;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use toml_edit::Document;

mod load;
mod statusline;

use crate::persist::write_private_atomically;
use crate::text::one_line;
pub(crate) use load::line_at;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Config {
    values: BTreeMap<&'static str, OptionValue>,
    undo_exclude: Option<Vec<String>>,
    statusline: StatusBarConfig,
}

impl Config {
    pub(crate) fn values(&self) -> &BTreeMap<&'static str, OptionValue> {
        &self.values
    }

    pub(crate) fn get(&self, name: &str) -> Option<&OptionValue> {
        options::canonical_name(name).and_then(|name| self.values.get(name))
    }

    pub(crate) fn undo_exclude(&self) -> Option<&[String]> {
        self.undo_exclude.as_deref()
    }

    pub(crate) fn statusline(&self) -> &StatusBarConfig {
        &self.statusline
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ConfigWarning {
    line: Option<usize>,
    key: String,
    value: String,
    fallback: String,
}

impl ConfigWarning {
    pub(crate) fn new(
        line: Option<usize>,
        key: impl Into<String>,
        value: impl Into<String>,
        fallback: impl Into<String>,
    ) -> Self {
        Self {
            line,
            key: key.into(),
            value: one_line(&value.into()),
            fallback: one_line(&fallback.into()),
        }
    }

    pub(crate) fn format(&self, path: &Path) -> String {
        let location = match self.line {
            Some(line) => format!("{}:{line}", path.display()),
            None => path.display().to_string(),
        };
        format!(
            "{location}: {}: rejected {}: using {}",
            self.key, self.value, self.fallback
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ConfigLoad {
    pub(crate) config: Config,
    pub(crate) warnings: Vec<ConfigWarning>,
}

pub(crate) fn load(path: &Path) -> ConfigLoad {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return ConfigLoad {
                config: Config::default(),
                warnings: Vec::new(),
            }
        }
        Err(error) => {
            return ConfigLoad {
                config: Config::default(),
                warnings: vec![ConfigWarning::new(
                    None,
                    "config",
                    error.to_string(),
                    "built-in defaults",
                )],
            }
        }
    };
    match Document::parse(text.as_str()) {
        Ok(document) => {
            let themes_dir = path.parent().map(|parent| parent.join("themes"));
            load::load_document(&text, &document, themes_dir.as_deref())
        }
        Err(error) => ConfigLoad {
            config: Config::default(),
            warnings: vec![ConfigWarning::new(
                load::line_at(&text, error.span()),
                "config",
                error.message(),
                "built-in defaults",
            )],
        },
    }
}

pub(crate) fn write_warning_log(log_path: &Path, warnings: &[String]) -> Result<()> {
    if warnings.is_empty() {
        return remove_stale_log(log_path);
    }
    let text = warnings.join("\n");
    write_private_atomically(log_path, format!("{text}\n").as_bytes())
}

fn remove_stale_log(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests;
