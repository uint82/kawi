use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use kawi_buffer::Buffer;
use kawi_plugin::PluginStatus;
use toml_edit::{DocumentMut, Item, Table};

use crate::app::App;
use crate::keymap::Flavor;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

pub(super) fn app() -> App {
    App::new(Buffer::from_string("alpha"), Flavor::Vim)
}

pub(super) fn temp_dir(name: &str) -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "kawi_app_plugin_{}_{}_{}",
        std::process::id(),
        name,
        id
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub(super) fn manifest(root: &std::path::Path) {
    let dir = root.join("echo");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        r#"name = "echo"
version = "0.1.0"
protocol = 1
command = ["/bin/true"]

[[commands]]
id = "ping"
title = "Ping"
description = "Return a greeting"

[commands.keybindings]
normal = ["<leader>pp"]
"#,
    )
    .unwrap();
}

pub(super) fn load_plugins(app: &mut App, root: &std::path::Path) -> Vec<String> {
    grant_all(root);
    load_plugins_with_permissions(
        app,
        root,
        &fs::read_to_string(root.join("permissions.toml")).unwrap(),
    )
}

pub(super) fn load_plugins_with_permissions(
    app: &mut App,
    root: &std::path::Path,
    permissions: &str,
) -> Vec<String> {
    write_permissions(root, permissions);
    app.plugins.load(
        crate::app::plugin::PluginLoadContext {
            dir: root,
            permissions: &root.join("permissions.toml"),
            workspace: root,
            commands: &mut app.commands,
            keymaps: &mut app.keymaps,
            leaders: &app.leaders,
            wake: app.events.wake(),
        },
        true,
    )
}

pub(super) fn write_permissions(root: &std::path::Path, permissions: &str) {
    let text = with_approved(root, permissions);
    fs::write(root.join("permissions.toml"), text).unwrap();
}

fn with_approved(root: &std::path::Path, permissions: &str) -> String {
    let Ok(mut document) = permissions.parse::<DocumentMut>() else {
        return permissions.to_string();
    };
    if document.get("plugins").is_none() {
        document.insert("plugins", Item::Table(Table::new()));
    }
    let catalog = kawi_plugin::load_catalog(root);
    for plugin in &catalog.plugins {
        let name = plugin.manifest.name.as_str();
        let Some(plugins) = document.get_mut("plugins").and_then(Item::as_table_mut) else {
            return permissions.to_string();
        };
        if !plugins.contains_key(name) {
            plugins.insert(name, Item::Table(Table::new()));
        }
        let Some(table) = plugins.get_mut(name).and_then(Item::as_table_mut) else {
            continue;
        };
        if table.contains_key("approved") || table.contains_key("request_hash") {
            continue;
        }
        let Ok(manifest_text) = fs::read_to_string(&plugin.path) else {
            continue;
        };
        let Ok(manifest) = manifest_text.parse::<DocumentMut>() else {
            continue;
        };
        let Some(capabilities) = manifest.get("capabilities").and_then(Item::as_table) else {
            continue;
        };
        let mut approved = Table::new();
        for (key, value) in capabilities.iter() {
            approved.insert(key, value.clone());
        }
        table.insert("approved", Item::Table(approved));
    }
    document.to_string()
}

pub(super) fn grant_all(root: &std::path::Path) {
    let mut text = String::new();
    for entry in fs::read_dir(root).unwrap().flatten() {
        let path = entry.path();
        let manifest_path = path.join("plugin.toml");
        if !manifest_path.is_file() {
            continue;
        }
        let manifest: toml::Value =
            toml::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
        let Some(capabilities) = manifest.get("capabilities").and_then(toml::Value::as_table)
        else {
            continue;
        };
        let name = path.file_name().unwrap().to_string_lossy();
        text.push_str(&format!("[plugins.{name}.grant]\n"));
        for capability in capabilities.keys() {
            text.push_str(&format!("\"{capability}\" = true\n"));
        }
        text.push('\n');
    }
    fs::write(root.join("permissions.toml"), text).unwrap();
}

pub(super) fn event_manifest(root: &std::path::Path, mode: &str, debounce: Option<u64>) -> PathBuf {
    let dir = root.join("events");
    fs::create_dir_all(&dir).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/app/plugin/test_data/event_plugin.py");
    fs::copy(source, dir.join("event_plugin.py")).unwrap();
    let debounce = debounce
        .map(|value| format!("[debounce]\nbuffer_changed = {value}\n\n"))
        .unwrap_or_default();
    fs::write(
        dir.join("plugin.toml"),
        format!(
            "name = \"events\"\nversion = \"0.1.0\"\nprotocol = 1\ncommand = [\"python3\", \"${{plugin_dir}}/event_plugin.py\", \"{mode}\"]\ntimeout_ms = 5000\nsubscribes = [\"buffer_changed\"]\n\n[capabilities]\n\"editor.read\" = true\n\"editor.write\" = true\n\n{debounce}[[activation_events]]\nevent = \"on_startup\"\n"
        ),
    )
    .unwrap();
    dir
}

pub(super) fn save_buffer(app: &mut App) -> bool {
    app.request_save(false, crate::app::save::SaveAfter::None, false)
}

pub(super) fn wait_for_plugin(app: &mut App, plugin: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.plugins.status(plugin) != Some(PluginStatus::Ready) && Instant::now() < deadline {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(app.plugins.status(plugin), Some(PluginStatus::Ready));
}

pub(super) fn denied_manifest(root: &std::path::Path) {
    let dir = root.join("denied");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        r#"name = "denied"
version = "0.1.0"
protocol = 1
command = ["/bin/true"]

[capabilities]
"editor.read" = true
"editor.write" = true
"ui.message" = true

[[commands]]
id = "ping"
title = "Ping"
"#,
    )
    .unwrap();
}

pub(super) fn gating_manifest(root: &std::path::Path) -> PathBuf {
    let dir = root.join("gating");
    fs::create_dir_all(&dir).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/app/plugin/test_data/gating_plugin.py");
    fs::copy(source, dir.join("gating_plugin.py")).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        r#"name = "gating"
version = "0.1.0"
protocol = 1
command = ["python3", "${plugin_dir}/gating_plugin.py"]
timeout_ms = 5000
subscribes = ["buffer_changed", "buffer_opened", "file_opened", "file_saved"]

[capabilities]
"editor.read" = true

[[activation_events]]
event = "on_startup"
"#,
    )
    .unwrap();
    dir
}

pub(super) fn denied_error(app: &App, method: &str) -> kawi_plugin::RpcError {
    let capability = app
        .plugins
        .denied_capability("denied", method)
        .expect("capability");
    app.plugins.capability_error("denied", capability)
}

pub(super) fn method_capability(method: &str) -> &'static str {
    match method {
        "editor.get_state" => "editor.read",
        "editor.apply_edit" => "editor.write",
        "editor.set_diagnostics" => "diagnostics.publish",
        "ui.message" => "ui.message",
        other => panic!("unexpected method: {other}"),
    }
}
