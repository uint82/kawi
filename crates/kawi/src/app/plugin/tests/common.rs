use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(feature = "wasm")]
use std::time::{Duration, Instant};

use kawi_buffer::Buffer;
use kawi_plugin::PluginLock;
use sha2::{Digest, Sha256};

use crate::app::App;
#[cfg(feature = "wasm")]
use crate::command::CommandHandler;
use crate::keymap::Flavor;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

const ECHO_COMPONENT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../kawi-plugin/tests/fixtures/wasm/echo.wasm"
);

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

pub(super) fn write_manifest(root: &Path, name: &str, version: &str, body: &str) -> PathBuf {
    let dir = root.join(name);
    fs::create_dir_all(&dir).unwrap();
    let component = fs::read(ECHO_COMPONENT).expect("wasm fixture");
    fs::write(dir.join("plugin.wasm"), &component).unwrap();
    let hash = Sha256::digest(&component);
    fs::write(
        dir.join("plugin.toml"),
        format!(
            r#"name = "{name}"
version = "{version}"
protocol = 1
runtime = "wasm"
component = "plugin.wasm"
sha256 = "sha256:{hash:x}"
{body}"#
        ),
    )
    .unwrap();
    dir
}

pub(super) fn manifest(root: &Path) {
    write_manifest(
        root,
        "echo",
        "0.1.0",
        r#"
[[commands]]
id = "ping"
title = "Ping"
description = "Return a greeting"

[commands.keybindings]
normal = ["<leader>pp"]
"#,
    );
}

pub(super) const CONFIGURABLE_PLUGIN: &str = "configurable";

pub(super) fn configurable_plugin(root: &Path) {
    write_manifest(
        root,
        CONFIGURABLE_PLUGIN,
        "0.1.0",
        r#"
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
    );
}

pub(super) fn load_settings_plugin(root: &Path) -> App {
    configurable_plugin(root);
    let mut app = app();
    let warnings = load_plugins(&mut app, root);
    assert!(warnings.is_empty(), "{warnings:?}");
    app
}

pub(super) fn configure_plugin_manager(app: &mut App, root: &Path) {
    app.plugin_manager.configure_for_test(
        root.join("installed"),
        root.to_path_buf(),
        root.join("plugins.lock"),
    );
}

pub(super) fn load_plugins(app: &mut App, root: &Path) -> Vec<String> {
    load_plugins_raw(app, root)
        .into_iter()
        .filter(|warning| !warning.contains("development plugin loaded"))
        .collect()
}

pub(super) fn load_plugins_raw(app: &mut App, root: &Path) -> Vec<String> {
    let lock = PluginLock::default();
    let warnings = app.plugins.load(
        crate::app::plugin::PluginLoadContext {
            installed: &root.join("installed"),
            dev: root,
            lock: &lock,
            workspace: root,
            settings: &root.join("plugins.toml"),
            commands: &mut app.commands,
            keymaps: &mut app.keymaps,
            leaders: &app.leaders,
            wake: app.events.wake(),
        },
        true,
    );
    app.rebuild_plugin_styles();
    warnings
}

#[cfg(feature = "wasm")]
pub(super) fn invoke_plugin(app: &mut App, name: &str) -> Option<String> {
    invoke_plugin_args(app, name, None)
}

#[cfg(feature = "wasm")]
pub(super) fn invoke_plugin_args(
    app: &mut App,
    name: &str,
    args: Option<toml::Value>,
) -> Option<String> {
    let command = app.commands.find(name).expect("plugin command");
    let target = match app.commands.spec(command).expect("command spec").handler {
        CommandHandler::Plugin(target) => target,
        other => panic!("unexpected handler: {other:?}"),
    };
    let plugin = app
        .plugins
        .target(target)
        .map(|(plugin, _)| plugin.to_string())
        .expect("plugin target");
    app.message = None;
    app.run_plugin_command(target, args).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        app.poll_plugins();
        if !app.plugins.in_flight.contains(&plugin) && app.message.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    app.message.clone()
}
