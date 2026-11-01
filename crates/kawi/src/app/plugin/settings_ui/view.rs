use kawi_picker::PickerRow;
use kawi_plugin::{PluginSetting, PluginSettingKind};

use super::super::text::sanitize;

pub(super) struct SettingEntry {
    pub(super) key: String,
    pub(super) setting: PluginSetting,
    pub(super) value: toml::Value,
}

impl SettingEntry {
    fn row(&self, index: usize) -> PickerRow {
        PickerRow {
            display: format!("{} = {}", self.key, display_value(&self.value)),
            path: String::new(),
            line: None,
            column: None,
            buffer: None,
            secondary: Some(self.details()),
            deprecated: false,
            id: Some(index as u64),
        }
    }

    fn details(&self) -> String {
        let mut details = Vec::new();
        push_description(&mut details, &self.setting);
        details.push(format!("type {}", kind_label(self.setting.kind)));
        details.push(format!("default {}", display_value(&self.setting.default)));
        push_choices(&mut details, &self.setting);
        push_range(&mut details, &self.setting);
        if self.value != self.setting.default {
            details.push("custom".to_string());
        }
        details.join(" | ")
    }
}

pub(super) fn setting_entries(
    entries: Vec<(String, PluginSetting, toml::Value)>,
) -> Vec<SettingEntry> {
    entries
        .into_iter()
        .map(|(key, setting, value)| SettingEntry {
            key,
            setting,
            value,
        })
        .collect()
}

pub(super) fn plugin_rows(plugins: Vec<(String, usize)>) -> Vec<PickerRow> {
    plugins
        .into_iter()
        .enumerate()
        .map(|(index, (name, count))| PickerRow {
            display: sanitize(&name),
            path: String::new(),
            line: None,
            column: None,
            buffer: None,
            secondary: Some(settings_label(count)),
            deprecated: false,
            id: Some(index as u64),
        })
        .collect()
}

pub(super) fn settings_rows(entries: &[SettingEntry]) -> Vec<PickerRow> {
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| entry.row(index))
        .collect()
}

pub(super) fn selected_index(entries: &[SettingEntry], selected: Option<&str>) -> Option<usize> {
    let selected = selected?;
    entries.iter().position(|entry| entry.key == selected)
}

pub(super) fn choice_rows(setting: &PluginSetting, current: &str) -> Vec<PickerRow> {
    setting
        .choices
        .iter()
        .enumerate()
        .map(|(index, choice)| PickerRow {
            display: sanitize(choice),
            path: String::new(),
            line: None,
            column: None,
            buffer: None,
            secondary: Some(if choice == current {
                "current".to_string()
            } else {
                "choice".to_string()
            }),
            deprecated: false,
            id: Some(index as u64),
        })
        .collect()
}

fn push_description(details: &mut Vec<String>, setting: &PluginSetting) {
    if !setting.description.is_empty() {
        details.push(sanitize(&setting.description));
    }
}

fn push_choices(details: &mut Vec<String>, setting: &PluginSetting) {
    if setting.choices.is_empty() {
        return;
    }
    details.push(format!(
        "choices {}",
        setting
            .choices
            .iter()
            .map(|choice| sanitize(choice))
            .collect::<Vec<_>>()
            .join(", ")
    ));
}

fn push_range(details: &mut Vec<String>, setting: &PluginSetting) {
    if setting.minimum.is_none() && setting.maximum.is_none() {
        return;
    }
    details.push(format!(
        "range {}{}",
        setting
            .minimum
            .as_ref()
            .map(display_value)
            .unwrap_or_else(|| "any".to_string()),
        setting
            .maximum
            .as_ref()
            .map(|value| format!("..{}", display_value(value)))
            .unwrap_or_default()
    ));
}

fn settings_label(count: usize) -> String {
    if count == 1 {
        "1 setting".to_string()
    } else {
        format!("{count} settings")
    }
}

fn kind_label(kind: PluginSettingKind) -> &'static str {
    match kind {
        PluginSettingKind::Boolean => "boolean",
        PluginSettingKind::Integer => "integer",
        PluginSettingKind::Number => "number",
        PluginSettingKind::String => "string",
    }
}

pub(super) fn display_value(value: &toml::Value) -> String {
    match value {
        toml::Value::String(value) => sanitize(value),
        _ => sanitize(&value.to_string()),
    }
}
