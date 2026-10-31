use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use kawi_buffer::Buffer;
use kawi_plugin::{PluginLock, PluginStatus};

use crate::app::App;
use crate::command::CommandHandler;
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

pub(super) fn copy_example_plugin(root: &Path, name: &str) -> PathBuf {
    let dir = root.join(name);
    fs::create_dir_all(&dir).unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/plugins")
        .join(name);
    fs::copy(source.join("plugin.toml"), dir.join("plugin.toml")).unwrap();
    fs::copy(source.join("plugin.py"), dir.join("plugin.py")).unwrap();
    dir
}

pub(super) fn configure_plugin_manager(app: &mut App, root: &std::path::Path) {
    app.plugin_manager.configure_for_test(
        root.join("installed"),
        root.to_path_buf(),
        root.join("plugins.lock"),
    );
}

pub(super) fn load_plugins(app: &mut App, root: &std::path::Path) -> Vec<String> {
    load_plugins_raw(app, root)
        .into_iter()
        .filter(|warning| !warning.contains("development plugin loaded"))
        .collect()
}

pub(super) fn load_plugins_raw(app: &mut App, root: &std::path::Path) -> Vec<String> {
    let lock = PluginLock::default();
    app.plugins.load(
        crate::app::plugin::PluginLoadContext {
            installed: &root.join("installed"),
            dev: root,
            lock: &lock,
            workspace: root,
            commands: &mut app.commands,
            keymaps: &mut app.keymaps,
            leaders: &app.leaders,
            wake: app.events.wake(),
        },
        true,
    )
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

pub(super) fn invoke_plugin(app: &mut App, name: &str) -> Option<String> {
    let command = app.commands.find(name).expect("plugin command");
    let target = match app.commands.spec(command).expect("command spec").handler {
        CommandHandler::Plugin(target) => target,
        other => panic!("unexpected handler: {other:?}"),
    };
    app.message = None;
    app.run_plugin_command(target, None).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.message.is_none() && Instant::now() < deadline {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    app.message.clone()
}
