use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use kawi_plugin::{PluginCatalog, PluginSetting};
use toml_edit::{Document, Item, Value};

use crate::config::line_at;
use crate::text::one_line;

#[derive(Default)]
pub(crate) struct PluginSettings {
    path: Option<PathBuf>,
    stamp: Option<SettingsStamp>,
    loaded: bool,
    values: BTreeMap<String, BTreeMap<String, toml::Value>>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SettingsStamp {
    modified: Option<SystemTime>,
    len: u64,
}

impl PluginSettings {
    pub(crate) fn set_path(&mut self, path: PathBuf) {
        if self.path.as_ref() == Some(&path) {
            return;
        }
        self.path = Some(path);
        self.loaded = false;
        self.stamp = None;
        self.values.clear();
        self.warnings.clear();
    }

    pub(crate) fn reload(&mut self, catalog: &PluginCatalog) -> Vec<String> {
        let had_values = self.loaded;
        self.loaded = true;
        let Some(path) = self.path.clone() else {
            return Vec::new();
        };
        match stamp(&path) {
            Ok(stamp) => {
                self.stamp = Some(stamp);
                self.load_file(&path, catalog);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.stamp = None;
                self.values = defaults(catalog);
                self.warnings.clear();
            }
            Err(error) => {
                if !had_values {
                    self.values = defaults(catalog);
                }
                self.warnings = vec![format!(
                    "{}: cannot read plugin settings: {error}",
                    path.display()
                )];
            }
        }
        self.warnings.clone()
    }

    pub(crate) fn ensure(&mut self, catalog: &PluginCatalog) -> Vec<String> {
        if !self.loaded {
            return self.reload(catalog);
        }
        let Some(path) = self.path.clone() else {
            return Vec::new();
        };
        match stamp(&path) {
            Ok(stamp) if self.stamp.as_ref() == Some(&stamp) => Vec::new(),
            Ok(_) => self.reload(catalog),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if self.stamp.is_none() {
                    Vec::new()
                } else {
                    self.reload(catalog)
                }
            }
            Err(error) => vec![format!(
                "{}: cannot read plugin settings: {error}",
                path.display()
            )],
        }
    }

    pub(crate) fn values(&self, plugin: &str) -> Option<&BTreeMap<String, toml::Value>> {
        self.values.get(plugin)
    }

    fn load_file(&mut self, path: &Path, catalog: &PluginCatalog) {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) => {
                self.values = defaults(catalog);
                self.warnings = vec![format!(
                    "{}: cannot read plugin settings: {error}",
                    path.display()
                )];
                return;
            }
        };
        match Document::parse(text.as_str()) {
            Ok(document) => {
                let mut values = defaults(catalog);
                let mut warnings = Vec::new();
                apply_document(path, &text, &document, catalog, &mut values, &mut warnings);
                self.values = values;
                self.warnings = warnings;
            }
            Err(error) => {
                self.values = defaults(catalog);
                self.warnings = vec![failure(
                    path,
                    line_at(&text, error.span()),
                    "settings",
                    error.message(),
                )];
            }
        }
    }
}

fn stamp(path: &Path) -> std::io::Result<SettingsStamp> {
    let metadata = fs::metadata(path)?;
    Ok(SettingsStamp {
        modified: metadata.modified().ok(),
        len: metadata.len(),
    })
}

fn defaults(catalog: &PluginCatalog) -> BTreeMap<String, BTreeMap<String, toml::Value>> {
    catalog
        .plugins
        .iter()
        .map(|plugin| {
            let values = plugin
                .manifest
                .settings
                .iter()
                .map(|(name, setting)| (name.clone(), setting.default.clone()))
                .collect();
            (plugin.manifest.name.clone(), values)
        })
        .collect()
}

fn apply_document(
    path: &Path,
    text: &str,
    document: &Document<&str>,
    catalog: &PluginCatalog,
    values: &mut BTreeMap<String, BTreeMap<String, toml::Value>>,
    warnings: &mut Vec<String>,
) {
    for (plugin_name, item) in document.iter() {
        let Some(table) = item.as_table() else {
            warnings.push(failure(
                path,
                line_at(text, item.span()),
                &format!("plugins.{plugin_name}"),
                "expected a table",
            ));
            continue;
        };
        let Some(plugin) = catalog
            .plugins
            .iter()
            .find(|plugin| plugin.manifest.name == plugin_name)
        else {
            warnings.push(ignored(
                path,
                line_at(text, item.span()),
                &format!("plugins.{plugin_name}"),
                "unknown plugin",
            ));
            continue;
        };
        let Some(plugin_values) = values.get_mut(plugin_name) else {
            continue;
        };
        apply_table(
            path,
            text,
            plugin_name,
            table,
            &plugin.manifest.settings,
            plugin_values,
            warnings,
        );
    }
}

fn apply_table(
    path: &Path,
    text: &str,
    plugin_name: &str,
    table: &toml_edit::Table,
    settings: &BTreeMap<String, PluginSetting>,
    values: &mut BTreeMap<String, toml::Value>,
    warnings: &mut Vec<String>,
) {
    for (name, item) in table.iter() {
        let key = format!("plugins.{plugin_name}.{name}");
        let Some(setting) = settings.get(name) else {
            warnings.push(ignored(
                path,
                line_at(text, item.span()),
                &key,
                "unknown setting",
            ));
            continue;
        };
        let Some(raw) = scalar(item) else {
            warnings.push(rejected(
                path,
                line_at(text, item.span()),
                &key,
                &item.to_string(),
                "expected a scalar",
                &display(&setting.default),
            ));
            continue;
        };
        match setting.normalize_value(&raw) {
            Ok(value) => {
                values.insert(name.to_string(), value);
            }
            Err(message) => warnings.push(rejected(
                path,
                line_at(text, item.span()),
                &key,
                &display(&raw),
                &message,
                &display(&setting.default),
            )),
        }
    }
}

fn scalar(item: &Item) -> Option<toml::Value> {
    match item.as_value()? {
        Value::String(value) => Some(toml::Value::String(value.value().clone())),
        Value::Integer(value) => Some(toml::Value::Integer(*value.value())),
        Value::Float(value) => Some(toml::Value::Float(*value.value())),
        Value::Boolean(value) => Some(toml::Value::Boolean(*value.value())),
        _ => None,
    }
}

fn display(value: &toml::Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| value.to_string())
}

fn rejected(
    path: &Path,
    line: Option<usize>,
    key: &str,
    value: &str,
    detail: &str,
    fallback: &str,
) -> String {
    format!(
        "{}: {key}: rejected {}: {detail}; using {}",
        location(path, line),
        one_line(value),
        one_line(fallback)
    )
}

fn ignored(path: &Path, line: Option<usize>, key: &str, detail: &str) -> String {
    format!("{}: {key}: {detail}", location(path, line))
}

fn failure(path: &Path, line: Option<usize>, key: &str, detail: &str) -> String {
    format!("{}: {key}: {}", location(path, line), one_line(detail))
}

pub(super) fn location(path: &Path, line: Option<usize>) -> String {
    match line {
        Some(line) => format!("{}:{line}", path.display()),
        None => path.display().to_string(),
    }
}

mod write;

#[cfg(test)]
mod tests;
