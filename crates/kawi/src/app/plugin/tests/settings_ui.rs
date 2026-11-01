use std::fs;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{json, Value};

use super::common::{app, load_plugins, load_settings_plugin, temp_dir, CONFIGURABLE_PLUGIN};
use crate::app::App;

const PLUGIN: &str = CONFIGURABLE_PLUGIN;

fn get(app: &mut App, key: &str) -> Option<Value> {
    let value = app
        .handle_plugin_request(PLUGIN, "config.get", json!({ "key": key }))
        .expect("config.get")["value"]
        .clone();
    (!value.is_null()).then_some(value)
}

fn load_toggle_plugin(root: &std::path::Path) -> App {
    let dir = root.join("toggle");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        r#"name = "toggle"
version = "0.1.0"
protocol = 1
command = ["/bin/true"]

[settings.enabled]
type = "boolean"
default = true
"#,
    )
    .unwrap();
    let mut app = app();
    let warnings = load_plugins(&mut app, root);
    assert!(warnings.is_empty(), "{warnings:?}");
    app
}

fn press(app: &mut App, code: KeyCode) {
    app.handle_plugin_settings_key(KeyEvent::new(code, KeyModifiers::NONE));
}

fn press_control(app: &mut App, character: char) {
    app.handle_plugin_settings_key(KeyEvent::new(
        KeyCode::Char(character),
        KeyModifiers::CONTROL,
    ));
}

fn type_text(app: &mut App, text: &str) {
    for character in text.chars() {
        press(app, KeyCode::Char(character));
    }
}

#[test]
fn command_lists_settings_and_opens_plugin_chooser() {
    let root = temp_dir("settings-list");
    let mut app = load_settings_plugin(&root);
    app.handle_plugin_command("settings configurable");
    let ui = app.plugin_settings.as_ref().expect("settings ui");
    let picker = ui.picker().expect("picker");
    assert_eq!(picker.title(), "Plugin settings: configurable");
    assert_eq!(picker.rows().len(), 2);
    assert!(picker.rows()[0].display.contains("mode = auto"));
    assert!(picker.rows()[1].display.contains("size = 10"));

    app.plugin_settings = None;
    app.handle_plugin_command("settings");
    let ui = app.plugin_settings.as_ref().expect("chooser");
    let picker = ui.picker().expect("picker");
    assert_eq!(picker.rows()[0].display, "configurable");
    press(&mut app, KeyCode::Enter);
    let ui = app.plugin_settings.as_ref().expect("settings ui");
    assert_eq!(
        ui.picker().expect("picker").title(),
        "Plugin settings: configurable"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn selects_an_enum_value_and_persists_it() {
    let root = temp_dir("settings-choice");
    let mut app = load_settings_plugin(&root);
    app.open_plugin_settings(PLUGIN);
    press(&mut app, KeyCode::Enter);
    let picker = app
        .plugin_settings
        .as_ref()
        .and_then(|ui| ui.picker())
        .expect("choices");
    assert_eq!(picker.title(), "configurable.mode");
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert_eq!(get(&mut app, "mode"), Some(json!("dark")));
    assert!(fs::read_to_string(root.join("plugins.toml"))
        .unwrap()
        .contains("mode = \"dark\""));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn edits_and_validates_an_integer() {
    let root = temp_dir("settings-integer");
    let mut app = load_settings_plugin(&root);
    app.open_plugin_settings(PLUGIN);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    let prompt = app
        .plugin_settings
        .as_ref()
        .and_then(|ui| ui.prompt())
        .expect("prompt");
    assert_eq!(prompt.value, "10");
    press_control(&mut app, 'u');
    type_text(&mut app, "0");
    press(&mut app, KeyCode::Enter);
    assert!(app
        .message
        .as_deref()
        .is_some_and(|message| message.contains("at least")));
    assert!(app
        .plugin_settings
        .as_ref()
        .and_then(|ui| ui.prompt())
        .is_some());

    press_control(&mut app, 'u');
    type_text(&mut app, "4");
    press(&mut app, KeyCode::Enter);
    assert_eq!(get(&mut app, "size"), Some(json!(4)));
    assert!(root.join("plugins.toml").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn resets_a_setting_to_its_default() {
    let root = temp_dir("settings-reset");
    let mut app = load_settings_plugin(&root);
    fs::write(
        root.join("plugins.toml"),
        "[configurable]\nmode = \"dark\"\n",
    )
    .unwrap();
    app.open_plugin_settings(PLUGIN);
    press_control(&mut app, 'r');
    assert_eq!(get(&mut app, "mode"), Some(json!("auto")));
    let text = fs::read_to_string(root.join("plugins.toml")).unwrap();
    assert!(!text.contains("mode"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unknown_plugin_reports_an_error() {
    let root = temp_dir("settings-unknown");
    let mut app = load_settings_plugin(&root);
    app.open_plugin_settings("missing");
    assert!(app.plugin_settings.is_none());
    assert!(app
        .message
        .as_deref()
        .is_some_and(|message| message.contains("Unknown plugin")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn toggles_a_boolean_and_removes_the_override_at_default() {
    let root = temp_dir("settings-boolean");
    let mut app = load_toggle_plugin(&root);
    app.open_plugin_settings("toggle");
    let picker = app
        .plugin_settings
        .as_ref()
        .and_then(|ui| ui.picker())
        .expect("picker");
    assert_eq!(picker.rows()[0].display, "enabled = true");
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        app.plugins.config_value("toggle", "enabled").unwrap(),
        Some(json!(false))
    );
    assert!(fs::read_to_string(root.join("plugins.toml"))
        .unwrap()
        .contains("enabled = false"));
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        app.plugins.config_value("toggle", "enabled").unwrap(),
        Some(json!(true))
    );
    let text = fs::read_to_string(root.join("plugins.toml")).unwrap();
    assert!(!text.contains("enabled"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn paste_filters_the_settings_picker() {
    let root = temp_dir("settings-paste");
    let mut app = load_settings_plugin(&root);
    app.open_plugin_settings(PLUGIN);
    assert!(app.paste_plugin_settings("size"));
    let picker = app
        .plugin_settings
        .as_ref()
        .and_then(|ui| ui.picker())
        .expect("picker");
    assert_eq!(picker.query(), "size");
    assert_eq!(picker.rows().len(), 1);
    assert!(picker.rows()[0].display.starts_with("size = "));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn reset_uses_the_filtered_selection() {
    let root = temp_dir("settings-reset-filtered");
    let mut app = load_settings_plugin(&root);
    fs::write(
        root.join("plugins.toml"),
        "[configurable]\nmode = \"dark\"\nsize = 4\n",
    )
    .unwrap();
    app.open_plugin_settings(PLUGIN);
    assert!(app.paste_plugin_settings("size"));
    press_control(&mut app, 'r');
    assert_eq!(get(&mut app, "size"), Some(json!(10)));
    assert_eq!(get(&mut app, "mode"), Some(json!("dark")));
    let text = fs::read_to_string(root.join("plugins.toml")).unwrap();
    assert!(text.contains("mode = \"dark\""));
    assert!(!text.contains("size"));
    fs::remove_dir_all(root).unwrap();
}
