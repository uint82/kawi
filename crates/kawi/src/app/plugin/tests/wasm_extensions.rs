use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use sha2::{Digest, Sha256};

use super::common::{app, invoke_plugin, invoke_plugin_args, load_plugins, temp_dir};

const MESSAGE_INTERVAL: Duration = Duration::from_millis(1100);

#[test]
fn wasm_plugins_register_and_unregister_runtime_commands() {
    let root = temp_dir("wasm-runtime-commands");
    write_plugin(&root);
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(
        invoke_plugin(&mut app, "wasm.ping"),
        Some("pong".to_string())
    );
    assert!(app.commands.find("wasm.greet").is_some());
    assert!(app.commands.find("hello").is_some());
    assert_eq!(app.plugins.registered_keymaps("wasm").len(), 1);
    std::thread::sleep(MESSAGE_INTERVAL);
    press(&mut app, ' ');
    press(&mut app, 'p');
    press(&mut app, 'g');
    wait_for_message(&mut app, "greet:none");
    std::thread::sleep(MESSAGE_INTERVAL);
    assert_eq!(
        invoke_plugin_args(
            &mut app,
            "wasm.greet",
            Some(toml::Value::String("world".to_string()))
        ),
        Some("greet:world".to_string()),
        "{:?}",
        app.plugins.logs("wasm")
    );
    std::thread::sleep(MESSAGE_INTERVAL);
    assert_eq!(
        invoke_plugin(&mut app, "wasm.unregister"),
        Some("unregistered".to_string())
    );
    assert!(app.commands.find("wasm.greet").is_none());
    assert!(app.commands.find("hello").is_none());
    assert!(app.plugins.registered_commands("wasm").is_empty());
    assert!(app.plugins.registered_keymaps("wasm").is_empty());
    fs::remove_dir_all(root).unwrap();
}

fn press(app: &mut crate::app::App, key: char) {
    app.handle_key(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE))
        .expect("key");
}

fn wait_for_message(app: &mut crate::app::App, expected: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.last_message() != Some(expected) {
        assert!(
            Instant::now() < deadline,
            "message timed out: {:?} {:?}",
            app.plugins.status("wasm"),
            app.plugins.logs("wasm")
        );
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn write_plugin(root: &Path) {
    let bytes = fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../kawi-plugin/tests/fixtures/wasm/commands.wasm"),
    )
    .unwrap();
    let dir = root.join("wasm");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("plugin.wasm"), &bytes).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        format!(
            r#"name = "wasm"
version = "0.1.0"
protocol = 1
runtime = "wasm"
component = "plugin.wasm"
sha256 = "sha256:{:x}"

[capabilities]
"editor.commands" = true
"editor.keymaps" = true
"ui.message" = true

[[commands]]
id = "ping"
title = "Ping"

[[commands]]
id = "unregister"
title = "Unregister"
capabilities = ["editor.commands"]
"#,
            Sha256::digest(&bytes)
        ),
    )
    .unwrap();
}

#[test]
fn wasm_autocmds_dispatch_matching_callbacks_and_stop_after_removal() {
    let root = temp_dir("wasm-autocmds");
    write_autocmd_plugin(&root);
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(
        invoke_plugin(&mut app, "wasm.register"),
        Some("0".to_string())
    );
    std::thread::sleep(MESSAGE_INTERVAL);
    app.plugins
        .file_saved(1, Some("src/main.rs".to_string()), 1);
    assert_eq!(wait_for_autocmd_count(&mut app, "1"), "1");
    std::thread::sleep(MESSAGE_INTERVAL);
    assert_eq!(
        invoke_plugin(&mut app, "wasm.last"),
        Some("0:1:src/main.rs".to_string())
    );
    std::thread::sleep(MESSAGE_INTERVAL);
    app.plugins
        .file_saved(1, Some("src/main.toml".to_string()), 2);
    assert_eq!(wait_for_autocmd_count(&mut app, "1"), "1");
    std::thread::sleep(MESSAGE_INTERVAL);
    assert_eq!(
        invoke_plugin(&mut app, "wasm.remove"),
        Some("removed".to_string())
    );
    std::thread::sleep(MESSAGE_INTERVAL);
    app.plugins
        .file_saved(1, Some("src/main.rs".to_string()), 3);
    assert_eq!(wait_for_autocmd_count(&mut app, "1"), "1");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn wasm_autocmds_receive_user_events() {
    let root = temp_dir("wasm-autocmd-user");
    write_autocmd_plugin(&root);
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(
        invoke_plugin(&mut app, "wasm.register-user"),
        Some("0".to_string())
    );
    std::thread::sleep(MESSAGE_INTERVAL);
    assert_eq!(invoke_plugin(&mut app, "wasm.emit"), None);
    assert_eq!(wait_for_autocmd_count(&mut app, "1"), "1");
    std::thread::sleep(MESSAGE_INTERVAL);
    assert_eq!(
        invoke_plugin(&mut app, "wasm.last"),
        Some("0:user:custom:data".to_string())
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn wasm_autocmd_once_registrations_fire_once() {
    let root = temp_dir("wasm-autocmd-once");
    write_autocmd_plugin(&root);
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(
        invoke_plugin(&mut app, "wasm.register-once"),
        Some("0".to_string())
    );
    std::thread::sleep(MESSAGE_INTERVAL);
    app.plugins
        .file_saved(1, Some("src/main.rs".to_string()), 1);
    assert_eq!(wait_for_autocmd_count(&mut app, "1"), "1");
    std::thread::sleep(MESSAGE_INTERVAL);
    app.plugins
        .file_saved(1, Some("src/main.rs".to_string()), 2);
    assert_eq!(wait_for_autocmd_count(&mut app, "1"), "1");
    fs::remove_dir_all(root).unwrap();
}

fn wait_for_autocmd_count(app: &mut crate::app::App, expected: &str) -> String {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(message) = invoke_plugin(app, "wasm.count") {
            if message == expected {
                return message;
            }
        }
        assert!(
            Instant::now() < deadline,
            "autocmd count timed out: {:?} {:?}",
            app.plugins.status("wasm"),
            app.plugins.logs("wasm")
        );
        app.poll_plugins();
        std::thread::sleep(MESSAGE_INTERVAL);
    }
}

fn write_autocmd_plugin(root: &Path) {
    let bytes = fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../kawi-plugin/tests/fixtures/wasm/autocmds.wasm"),
    )
    .unwrap();
    let dir = root.join("wasm");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("plugin.wasm"), &bytes).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        format!(
            r#"name = "wasm"
version = "0.1.0"
protocol = 1
runtime = "wasm"
component = "plugin.wasm"
sha256 = "sha256:{:x}"

[capabilities]
"editor.read" = true
"editor.autocmds" = true
"ui.message" = true

[[commands]]
id = "register"
title = "Register"

[[commands]]
id = "register-once"
title = "Register once"

[[commands]]
id = "register-user"
title = "Register user"

[[commands]]
id = "remove"
title = "Remove"

[[commands]]
id = "count"
title = "Count"

[[commands]]
id = "last"
title = "Last"

[[commands]]
id = "emit"
title = "Emit"

[[commands]]
id = "clear"
title = "Clear"
"#,
            Sha256::digest(&bytes)
        ),
    )
    .unwrap();
}
