use std::fs;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;

use super::common::*;
use crate::app::App;
use crate::keymap::Flavor;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn delivers_prompt_responses() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let root = temp_dir("ui_response");
    let dir = root.join("ui");
    fs::create_dir_all(&dir).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/app/plugin/test_data/ui_plugin.py");
    fs::copy(source, dir.join("ui_plugin.py")).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        "name = \"ui\"\nversion = \"0.1.0\"\nprotocol = 1\ncommand = [\"python3\", \"${plugin_dir}/ui_plugin.py\"]\ntimeout_ms = 1000\nsubscribes = [\"ui_response\"]\n\n[capabilities]\n\"ui.prompt\" = true\n\n[[commands]]\nid = \"prompt\"\ntitle = \"Prompt\"\n\n[[activation_events]]\nevent = \"on_startup\"\n",
    )
    .unwrap();
    let mut app = App::new(Buffer::from_string("alpha"), Flavor::Vim);
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.plugins.status("ui") != Some(kawi_plugin::PluginStatus::Ready)
        && Instant::now() < deadline
    {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        app.plugins.status("ui"),
        Some(kawi_plugin::PluginStatus::Ready)
    );
    let command = app.commands.find("ui.prompt").expect("plugin command");
    let target = match app.commands.spec(command).expect("command spec").handler {
        crate::command::CommandHandler::Plugin(target) => target,
        other => panic!("unexpected handler: {other:?}"),
    };
    app.run_plugin_command(target, None).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !app.plugin_ui.has_prompt() && Instant::now() < deadline {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(app.plugin_ui.has_prompt());
    std::thread::sleep(Duration::from_millis(1100));
    for character in "hello".chars() {
        app.handle_key(key(KeyCode::Char(character))).unwrap();
    }
    app.handle_key(key(KeyCode::Enter)).unwrap();
    let log = dir.join("ui-response.log");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !log.exists() && Instant::now() < deadline {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    let payload: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(log).unwrap()).unwrap();
    assert_eq!(payload["kind"], "text");
    assert_eq!(payload["value"], "hello");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cancels_plugin_progress() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let root = temp_dir("ui_progress");
    let dir = root.join("ui");
    fs::create_dir_all(&dir).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/app/plugin/test_data/ui_plugin.py");
    fs::copy(source, dir.join("ui_plugin.py")).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        "name = \"ui\"\nversion = \"0.1.0\"\nprotocol = 1\ncommand = [\"python3\", \"${plugin_dir}/ui_plugin.py\"]\ntimeout_ms = 1000\n\n[capabilities]\n\"ui.prompt\" = true\n\"ui.progress\" = true\n\n[[commands]]\nid = \"progress\"\ntitle = \"Progress\"\n\n[[activation_events]]\nevent = \"on_startup\"\n",
    )
    .unwrap();
    let mut app = App::new(Buffer::from_string("alpha"), Flavor::Vim);
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    wait_for_plugin(&mut app, "ui");
    let command = app.commands.find("ui.progress").expect("plugin command");
    let target = match app.commands.spec(command).expect("command spec").handler {
        crate::command::CommandHandler::Plugin(target) => target,
        other => panic!("unexpected handler: {other:?}"),
    };
    app.run_plugin_command(target, None).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.plugin_ui.progress_text().is_none() && Instant::now() < deadline {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(app.plugin_ui.progress_text().is_some());
    app.execute_command_text("plugin cancel").unwrap();
    let log = dir.join("ui-response.log");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !log.exists() && Instant::now() < deadline {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    let payload: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(log).unwrap()).unwrap();
    assert_eq!(payload["canceled"], true);
    fs::remove_dir_all(root).unwrap();
}
