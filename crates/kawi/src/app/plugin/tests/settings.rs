use std::fs;

use kawi_plugin::INVALID_PARAMS;
use serde_json::{json, Value};

use super::common::{app, load_plugins, manifest, temp_dir};
use crate::app::App;

const PLUGIN: &str = "configurable";

fn write_plugin(root: &std::path::Path) {
    let dir = root.join(PLUGIN);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        r#"name = "configurable"
version = "0.1.0"
protocol = 1
command = ["/bin/true"]

[settings.size]
type = "integer"
default = 10
minimum = 1
maximum = 20

[settings.mode]
type = "string"
default = "auto"
enum = ["auto", "dark"]
"#,
    )
    .unwrap();
}

fn load(root: &std::path::Path) -> App {
    write_plugin(root);
    let mut app = app();
    let warnings = load_plugins(&mut app, root);
    assert!(warnings.is_empty(), "{warnings:?}");
    app
}

fn get(app: &mut App, key: &str) -> Option<Value> {
    let value = app
        .handle_plugin_request(PLUGIN, "config.get", json!({ "key": key }))
        .expect("config.get")["value"]
        .clone();
    (!value.is_null()).then_some(value)
}

fn list(app: &mut App) -> Vec<(String, Value)> {
    let value = app
        .handle_plugin_request(PLUGIN, "config.list", Value::Null)
        .expect("config.list");
    value["settings"]
        .as_array()
        .expect("settings")
        .iter()
        .map(|setting| {
            (
                setting["key"].as_str().expect("key").to_string(),
                setting["value"].clone(),
            )
        })
        .collect()
}

#[test]
fn gets_defaults_and_missing_values() {
    let root = temp_dir("config-get");
    let mut app = load(&root);
    assert_eq!(get(&mut app, "size"), Some(json!(10)));
    assert_eq!(get(&mut app, "mode"), Some(json!("auto")));
    assert_eq!(get(&mut app, "missing"), None);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn applies_user_values_and_lists_them_sorted() {
    let root = temp_dir("config-list");
    fs::write(
        root.join("plugins.toml"),
        "[configurable]\nmode = \"dark\"\nsize = 4\n",
    )
    .unwrap();
    let mut app = load(&root);
    assert_eq!(get(&mut app, "size"), Some(json!(4)));
    assert_eq!(
        list(&mut app),
        vec![
            ("mode".to_string(), json!("dark")),
            ("size".to_string(), json!(4)),
        ]
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn reloads_changed_settings_without_restarting_the_plugin() {
    let root = temp_dir("config-reload");
    let mut app = load(&root);
    assert_eq!(get(&mut app, "size"), Some(json!(10)));
    fs::write(root.join("plugins.toml"), "[configurable]\nsize = 7\n").unwrap();
    assert_eq!(get(&mut app, "size"), Some(json!(7)));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn inspect_lists_declared_settings() {
    let root = temp_dir("config-inspect");
    let mut app = load(&root);
    app.handle_plugin_command("inspect configurable");
    let lines = app.pager.as_ref().expect("pager");
    assert!(
        lines.iter().any(|line| line == "settings: mode, size"),
        "{lines:?}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_values_fall_back_and_are_logged() {
    let root = temp_dir("config-invalid");
    write_plugin(&root);
    fs::write(root.join("plugins.toml"), "[configurable]\nsize = 0\n").unwrap();
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(
        warnings.iter().any(|warning| warning.contains("size")),
        "{warnings:?}"
    );
    assert_eq!(get(&mut app, "size"), Some(json!(10)));
    fs::write(
        root.join("plugins.toml"),
        "[configurable]\nsize = 0\nmode = \"dark\"\n",
    )
    .unwrap();
    assert_eq!(get(&mut app, "size"), Some(json!(10)));
    assert!(app
        .plugins
        .logs(PLUGIN)
        .iter()
        .any(|line| line.contains("size")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn plugins_without_settings_return_empty_values() {
    let root = temp_dir("config-none");
    manifest(&root);
    let mut app = app();
    load_plugins(&mut app, &root);
    assert_eq!(
        app.handle_plugin_request("echo", "config.list", Value::Null)
            .expect("config.list"),
        json!({ "settings": [] })
    );
    assert_eq!(
        app.handle_plugin_request("echo", "config.get", json!({ "key": "missing" }))
            .expect("config.get"),
        json!({ "value": null })
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_invalid_setting_keys() {
    let root = temp_dir("config-key");
    let mut app = load(&root);
    for key in ["", &"x".repeat(65), "Bad", "with space"] {
        let error = app
            .handle_plugin_request(PLUGIN, "config.get", json!({ "key": key }))
            .expect_err(key);
        assert_eq!(error.code, INVALID_PARAMS);
    }
    fs::remove_dir_all(root).unwrap();
}
