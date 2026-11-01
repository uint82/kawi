use std::fs;
use std::path::Path;

use kawi_plugin::PluginCatalog;
use toml_edit::{DocumentMut, Item, Table, Value};

use super::{location, PluginSettings};
use crate::config::line_at;
use crate::persist::write_private_atomically;
use crate::text::one_line;

impl PluginSettings {
    pub(crate) fn set_value(
        &mut self,
        catalog: &PluginCatalog,
        plugin: &str,
        key: &str,
        value: toml::Value,
    ) -> Result<(), String> {
        let setting = catalog
            .plugins
            .iter()
            .find(|entry| entry.manifest.name == plugin)
            .and_then(|entry| entry.manifest.settings.get(key))
            .ok_or_else(|| format!("unknown plugin setting: {plugin}.{key}"))?;
        let value = setting.normalize_value(&value)?;
        let path = self
            .path
            .clone()
            .ok_or_else(|| "plugin settings path is not configured".to_string())?;
        let mut document = read_document(&path)?;
        if !apply_value(&mut document, plugin, key, &value, &setting.default)? {
            return Ok(());
        }
        persist(&path, &document)?;
        self.reload(catalog);
        Ok(())
    }
}

fn persist(path: &Path, document: &DocumentMut) -> Result<(), String> {
    write_private_atomically(path, document.to_string().as_bytes())
        .map_err(|error| format!("{}: cannot write plugin settings: {error}", path.display()))
}

fn apply_value(
    document: &mut DocumentMut,
    plugin: &str,
    key: &str,
    value: &toml::Value,
    default: &toml::Value,
) -> Result<bool, String> {
    if value == default {
        return remove_override(document, plugin, key);
    }
    set_override(document, plugin, key, value)?;
    Ok(true)
}

fn read_document(path: &Path) -> Result<DocumentMut, String> {
    match fs::read_to_string(path) {
        Ok(text) => text.parse::<DocumentMut>().map_err(|error| {
            format!(
                "{}: cannot edit plugin settings: {}",
                location(path, line_at(&text, error.span())),
                one_line(error.message())
            )
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(DocumentMut::new()),
        Err(error) => Err(format!(
            "{}: cannot read plugin settings: {error}",
            path.display()
        )),
    }
}

fn set_override(
    document: &mut DocumentMut,
    plugin: &str,
    key: &str,
    value: &toml::Value,
) -> Result<(), String> {
    let table = document
        .entry(plugin)
        .or_insert(Item::Table(Table::new()))
        .as_table_mut()
        .ok_or_else(|| format!("plugins.{plugin} must be a table"))?;
    table.insert(key, Item::Value(edit_value(value)?));
    Ok(())
}

fn remove_override(document: &mut DocumentMut, plugin: &str, key: &str) -> Result<bool, String> {
    let Some(item) = document.get_mut(plugin) else {
        return Ok(false);
    };
    let table = item
        .as_table_mut()
        .ok_or_else(|| format!("plugins.{plugin} must be a table"))?;
    let removed = table.remove(key).is_some();
    if table.is_empty() {
        document.remove(plugin);
    }
    Ok(removed)
}

fn edit_value(value: &toml::Value) -> Result<Value, String> {
    match value {
        toml::Value::String(value) => Ok(Value::from(value.as_str())),
        toml::Value::Integer(value) => Ok(Value::from(*value)),
        toml::Value::Float(value) => Ok(Value::from(*value)),
        toml::Value::Boolean(value) => Ok(Value::from(*value)),
        _ => Err("setting values must be scalar".to_string()),
    }
}
