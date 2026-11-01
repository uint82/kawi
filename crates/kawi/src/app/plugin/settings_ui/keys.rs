use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_picker::{Picker, PickerOutcome};
use kawi_plugin::{PluginSetting, PluginSettingKind};

use super::view::SettingEntry;
use super::{PluginSettingsUi, View, MAX_VALUE_CHARS};
use crate::app::text_input::{handle_text_input, TextInputOutcome};
use crate::app::App;

impl App {
    pub(crate) fn open_plugin_settings(&mut self, name: &str) {
        if self.plugin_ui.has_prompt() {
            self.show_message("Close the plugin prompt before editing settings".to_string());
            return;
        }
        let name = name.trim();
        if name.is_empty() {
            let plugins = self.plugins.plugins_with_settings();
            if plugins.is_empty() {
                self.show_message("No plugins declare settings".to_string());
            } else {
                self.plugin_settings = Some(PluginSettingsUi::plugins(plugins));
            }
            return;
        }
        self.open_plugin_settings_for(name, None);
    }

    pub(crate) fn handle_plugin_settings_key(&mut self, event: KeyEvent) {
        let Some(mut ui) = self.plugin_settings.take() else {
            return;
        };
        if ui.handle_key(self, event) {
            self.plugin_settings = Some(ui);
        }
    }

    pub(crate) fn paste_plugin_settings(&mut self, text: &str) -> bool {
        self.plugin_settings
            .as_mut()
            .is_some_and(|ui| ui.paste(text))
    }

    pub(super) fn open_plugin_settings_for(&mut self, plugin: &str, selected: Option<&str>) {
        let Some(entries) = self.plugins.settings_entries(plugin) else {
            self.show_message(format!(
                "Unknown plugin: {}",
                super::super::text::sanitize(plugin)
            ));
            return;
        };
        if entries.is_empty() {
            self.show_message(format!(
                "Plugin {} declares no settings",
                super::super::text::sanitize(plugin)
            ));
            return;
        }
        self.plugin_settings = Some(PluginSettingsUi::settings(plugin, entries, selected));
    }

    fn edit_setting(&mut self, plugin: &str, entry: &SettingEntry) {
        if entry.setting.kind == PluginSettingKind::Boolean {
            self.toggle_setting(plugin, entry);
        } else if entry.setting.choices.is_empty() {
            self.open_value_editor(plugin, entry);
        } else {
            self.open_choice_editor(plugin, entry);
        }
    }

    fn toggle_setting(&mut self, plugin: &str, entry: &SettingEntry) {
        let toml::Value::Boolean(value) = entry.value else {
            self.show_message(format!("{} is not a boolean", entry.key));
            self.open_plugin_settings_for(plugin, Some(&entry.key));
            return;
        };
        if !self.save_setting(plugin, &entry.key, toml::Value::Boolean(!value)) {
            self.open_plugin_settings_for(plugin, Some(&entry.key));
        }
    }

    fn open_choice_editor(&mut self, plugin: &str, entry: &SettingEntry) {
        let current = match &entry.value {
            toml::Value::String(value) => value.as_str(),
            _ => "",
        };
        self.plugin_settings = Some(PluginSettingsUi::choice(
            plugin,
            &entry.key,
            entry.setting.clone(),
            current,
        ));
    }

    fn open_value_editor(&mut self, plugin: &str, entry: &SettingEntry) {
        self.plugin_settings = Some(PluginSettingsUi::value(
            plugin,
            &entry.key,
            entry.setting.clone(),
            &entry.value,
        ));
    }

    fn save_setting(&mut self, plugin: &str, key: &str, value: toml::Value) -> bool {
        match self.plugins.set_setting(plugin, key, value) {
            Ok(()) => {
                self.show_message(format!(
                    "Saved {}.{}",
                    super::super::text::sanitize(plugin),
                    key
                ));
                self.open_plugin_settings_for(plugin, Some(key));
                true
            }
            Err(error) => {
                self.show_message(super::super::text::sanitize(&error));
                false
            }
        }
    }
}

impl PluginSettingsUi {
    fn handle_key(&mut self, app: &mut App, event: KeyEvent) -> bool {
        match &mut self.view {
            View::Plugins { plugins, picker } => handle_plugins(app, plugins, picker, event),
            View::Settings {
                plugin,
                entries,
                picker,
            } => handle_settings(app, plugin, entries, picker, event),
            View::Choice {
                plugin,
                key,
                setting,
                picker,
            } => handle_choice(app, plugin, key, setting, picker, event),
            View::Value {
                plugin,
                key,
                setting,
                input,
                ..
            } => handle_value(app, plugin, key, setting, input, event),
        }
    }
}

fn handle_plugins(app: &mut App, plugins: &[String], picker: &mut Picker, event: KeyEvent) -> bool {
    match picker.handle_key(event) {
        PickerOutcome::Cancel => false,
        PickerOutcome::Open(row) => {
            let Some(plugin) = plugins.get(row.id.unwrap_or_default() as usize) else {
                return true;
            };
            app.open_plugin_settings_for(plugin, None);
            false
        }
        PickerOutcome::Consumed | PickerOutcome::QueryChanged => true,
    }
}

fn handle_settings(
    app: &mut App,
    plugin: &str,
    entries: &[SettingEntry],
    picker: &mut Picker,
    event: KeyEvent,
) -> bool {
    if is_reset_key(event) {
        let Some(entry) = selected_entry(picker, entries) else {
            return true;
        };
        let value = entry.setting.default.clone();
        return !app.save_setting(plugin, &entry.key, value);
    }
    match picker.handle_key(event) {
        PickerOutcome::Cancel => false,
        PickerOutcome::Open(row) => {
            let Some(entry) = entries.get(row.id.unwrap_or_default() as usize) else {
                return true;
            };
            app.edit_setting(plugin, entry);
            false
        }
        PickerOutcome::Consumed | PickerOutcome::QueryChanged => true,
    }
}

fn handle_choice(
    app: &mut App,
    plugin: &str,
    key: &str,
    setting: &PluginSetting,
    picker: &mut Picker,
    event: KeyEvent,
) -> bool {
    match picker.handle_key(event) {
        PickerOutcome::Cancel => {
            app.open_plugin_settings_for(plugin, Some(key));
            false
        }
        PickerOutcome::Open(row) => {
            if let Some(choice) = setting.choices.get(row.id.unwrap_or_default() as usize) {
                if !app.save_setting(plugin, key, toml::Value::String(choice.clone())) {
                    app.open_plugin_settings_for(plugin, Some(key));
                }
            } else {
                app.open_plugin_settings_for(plugin, Some(key));
            }
            false
        }
        PickerOutcome::Consumed | PickerOutcome::QueryChanged => true,
    }
}

fn handle_value(
    app: &mut App,
    plugin: &str,
    key: &str,
    setting: &PluginSetting,
    input: &mut kawi_picker::TextInput,
    event: KeyEvent,
) -> bool {
    match handle_text_input(input, event, MAX_VALUE_CHARS) {
        TextInputOutcome::Cancel => {
            app.open_plugin_settings_for(plugin, Some(key));
            false
        }
        TextInputOutcome::Submit => match parse_value(setting, input.text()) {
            Ok(value) => !app.save_setting(plugin, key, value),
            Err(message) => {
                app.show_message(message);
                true
            }
        },
        TextInputOutcome::Consumed => true,
    }
}

fn parse_value(setting: &PluginSetting, text: &str) -> Result<toml::Value, String> {
    let value = match setting.kind {
        PluginSettingKind::Boolean => toml::Value::Boolean(
            text.trim()
                .parse()
                .map_err(|_| "expected true or false".to_string())?,
        ),
        PluginSettingKind::Integer => toml::Value::Integer(
            text.trim()
                .parse()
                .map_err(|_| "expected an integer".to_string())?,
        ),
        PluginSettingKind::Number => {
            let value: f64 = text
                .trim()
                .parse()
                .map_err(|_| "expected a number".to_string())?;
            if !value.is_finite() {
                return Err("expected a finite number".to_string());
            }
            toml::Value::Float(value)
        }
        PluginSettingKind::String => toml::Value::String(text.to_string()),
    };
    setting.normalize_value(&value)
}

fn selected_entry<'a>(picker: &Picker, entries: &'a [SettingEntry]) -> Option<&'a SettingEntry> {
    let row = picker.rows().get(picker.selected()?)?;
    entries.get(row.id? as usize)
}

fn is_reset_key(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('r')
}
