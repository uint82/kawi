use kawi_picker::{Picker, TextInput};
use kawi_plugin::PluginSetting;
use kawi_tui::frame::PromptMenu;

use super::text::sanitize;

pub(super) const MAX_VALUE_CHARS: usize = 4096;
const STATUS: &str = "Enter edit | Ctrl+R reset | Esc close";

mod keys;
mod view;

use view::{
    choice_rows, display_value, plugin_rows, selected_index, setting_entries, settings_rows,
    SettingEntry,
};

pub(crate) struct PluginSettingsUi {
    view: View,
}

enum View {
    Plugins {
        plugins: Vec<String>,
        picker: Picker,
    },
    Settings {
        plugin: String,
        entries: Vec<SettingEntry>,
        picker: Picker,
    },
    Choice {
        plugin: String,
        key: String,
        setting: PluginSetting,
        picker: Picker,
    },
    Value {
        plugin: String,
        key: String,
        label: String,
        setting: PluginSetting,
        input: TextInput,
    },
}

impl PluginSettingsUi {
    pub(crate) fn plugins(plugins: Vec<(String, usize)>) -> Self {
        let names = plugins.iter().map(|(name, _)| name.clone()).collect();
        let rows = plugin_rows(plugins);
        let mut picker = Picker::locations("Plugin settings", rows);
        picker.set_status(Some("Enter select | Esc close".to_string()));
        Self {
            view: View::Plugins {
                plugins: names,
                picker,
            },
        }
    }

    pub(crate) fn settings(
        plugin: &str,
        entries: Vec<(String, PluginSetting, toml::Value)>,
        selected: Option<&str>,
    ) -> Self {
        let entries = setting_entries(entries);
        let mut picker = Picker::locations(
            format!("Plugin settings: {plugin}"),
            settings_rows(&entries),
        );
        picker.set_status(Some(STATUS.to_string()));
        if let Some(index) = selected_index(&entries, selected) {
            picker.select_symbol(index as u64);
        }
        Self {
            view: View::Settings {
                plugin: plugin.to_string(),
                entries,
                picker,
            },
        }
    }

    pub(super) fn choice(plugin: &str, key: &str, setting: PluginSetting, current: &str) -> Self {
        let rows = choice_rows(&setting, current);
        let mut picker = Picker::locations(format!("{plugin}.{key}"), rows);
        picker.set_status(Some("Enter choose | Esc back".to_string()));
        if let Some(index) = setting.choices.iter().position(|choice| choice == current) {
            picker.select_symbol(index as u64);
        }
        Self {
            view: View::Choice {
                plugin: plugin.to_string(),
                key: key.to_string(),
                setting,
                picker,
            },
        }
    }

    pub(super) fn value(
        plugin: &str,
        key: &str,
        setting: PluginSetting,
        value: &toml::Value,
    ) -> Self {
        let mut input = TextInput::new();
        input.set_text(&display_value(value));
        Self {
            view: View::Value {
                plugin: plugin.to_string(),
                key: key.to_string(),
                label: format!("{plugin}.{key}"),
                setting,
                input,
            },
        }
    }

    pub(crate) fn picker(&self) -> Option<&Picker> {
        match &self.view {
            View::Plugins { picker, .. }
            | View::Settings { picker, .. }
            | View::Choice { picker, .. } => Some(picker),
            View::Value { .. } => None,
        }
    }

    pub(crate) fn prompt(&self) -> Option<PromptMenu<'_>> {
        match &self.view {
            View::Value { label, input, .. } => Some(PromptMenu {
                label,
                value: input.text(),
                cursor: input.cursor(),
            }),
            _ => None,
        }
    }

    pub(crate) fn paste(&mut self, text: &str) -> bool {
        match &mut self.view {
            View::Value { input, .. } => {
                super::super::text_input::insert_text(input, text, MAX_VALUE_CHARS);
            }
            View::Plugins { picker, .. }
            | View::Settings { picker, .. }
            | View::Choice { picker, .. } => picker.insert_text(&sanitize(text)),
        }
        true
    }
}
