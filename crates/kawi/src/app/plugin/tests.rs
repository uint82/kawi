use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use kawi_buffer::Buffer;
use kawi_editor::Action;
use kawi_plugin::{
    ApplyEditParams, GetStateParams, PluginStatus, Position, ProtocolTextEdit, INVALID_POSITION,
    STALE_REVISION,
};
use serde_json::json;

use super::super::App;
use crate::command::{CommandHandler, CommandSource};
use crate::keymap::{BindingSource, Flavor};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn app() -> App {
    App::new(Buffer::from_string("alpha"), Flavor::Vim)
}

fn temp_dir(name: &str) -> PathBuf {
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

fn manifest(root: &std::path::Path) {
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

fn load_plugins(app: &mut App, root: &std::path::Path) -> Vec<String> {
    app.plugins.load(
        root,
        &mut app.commands,
        &mut app.keymaps,
        &app.leaders,
        app.events.wake(),
    )
}

#[test]
fn static_contributions_register_commands_and_bindings() {
    let root = temp_dir("static");
    manifest(&root);
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    let command = app.commands.find("echo.ping").expect("plugin command");
    let spec = app.commands.spec(command).expect("spec");
    assert_eq!(spec.handler, CommandHandler::Plugin(0));
    assert_eq!(spec.source, CommandSource::Plugin("echo".to_string()));
    assert_eq!(app.plugins.target(0), Some(("echo", "ping")));
    assert_eq!(app.keymaps.bindings().len(), 2);
    assert!(app
        .keymaps
        .bindings()
        .iter()
        .all(|binding| binding.source == BindingSource::Plugin));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn keymap_reload_keeps_plugin_defaults() {
    let root = temp_dir("reload");
    manifest(&root);
    let mut app = app();
    load_plugins(&mut app, &root);
    app.apply_keymaps(crate::keymap::config::KeymapLoad::default());
    assert_eq!(app.keymaps.bindings().len(), 2);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn nunmap_does_not_remove_plugin_defaults() {
    let root = temp_dir("nunmap");
    manifest(&root);
    let mut app = app();
    load_plugins(&mut app, &root);
    app.handle_map_command("nunmap <leader>pp").unwrap();
    assert_eq!(app.keymaps.bindings().len(), 2);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn lists_plugins_without_starting_them() {
    let root = temp_dir("list");
    manifest(&root);
    let mut app = app();
    load_plugins(&mut app, &root);
    app.handle_plugin_command("list");
    let lines = app.pager.as_ref().expect("pager");
    assert!(lines[0].contains("echo"));
    assert!(lines[0].contains("stopped"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn get_state_uses_utf8_byte_columns() {
    let mut app = App::new(Buffer::from_string("héllo"), Flavor::Vim);
    app.editor.apply(Action::MoveRight);
    app.editor.apply(Action::MoveRight);
    let params = serde_json::to_value(GetStateParams { include_text: true }).unwrap();
    let value = app
        .handle_plugin_request("editor.get_state", params)
        .unwrap();
    assert_eq!(value["text"], "héllo");
    assert_eq!(value["cursor"]["line"], 0);
    assert_eq!(value["cursor"]["byte_column"], 3);
    assert_eq!(value["mode"], "normal");
}

#[test]
fn applies_edits_at_the_current_revision() {
    let mut app = App::new(Buffer::from_string("héllo"), Flavor::Vim);
    let revision = app.editor.buffer_revision();
    let params = ApplyEditParams {
        buffer: app.editor.buffer_number() as u64,
        revision,
        edits: vec![ProtocolTextEdit {
            start: Position {
                line: 0,
                byte_column: 1,
            },
            end: Position {
                line: 0,
                byte_column: 3,
            },
            text: "i".to_string(),
        }],
    };
    let value = app
        .handle_plugin_request("editor.apply_edit", serde_json::to_value(params).unwrap())
        .unwrap();
    assert_eq!(app.editor.text_lines(), vec!["hillo"]);
    assert_eq!(value["revision"], revision + 1);
}

#[test]
fn rejects_stale_revisions_and_split_code_points() {
    let mut app = app();
    let stale = ApplyEditParams {
        buffer: app.editor.buffer_number() as u64,
        revision: 99,
        edits: Vec::new(),
    };
    let error = app
        .handle_plugin_request("editor.apply_edit", serde_json::to_value(stale).unwrap())
        .unwrap_err();
    assert_eq!(error.code, STALE_REVISION);
    assert_eq!(app.editor.text_lines(), vec!["alpha"]);

    let wrong_buffer = ApplyEditParams {
        buffer: app.editor.buffer_number() as u64 + 100,
        revision: app.editor.buffer_revision(),
        edits: Vec::new(),
    };
    let error = app
        .handle_plugin_request(
            "editor.apply_edit",
            serde_json::to_value(wrong_buffer).unwrap(),
        )
        .unwrap_err();
    assert_eq!(error.code, STALE_REVISION);
    assert!(error.message.contains("stale buffer"));

    for text in ["é", "中", "😀"] {
        let mut app = App::new(Buffer::from_string(text), Flavor::Vim);
        let revision = app.editor.buffer_revision();
        let split = ApplyEditParams {
            buffer: app.editor.buffer_number() as u64,
            revision,
            edits: vec![ProtocolTextEdit {
                start: Position {
                    line: 0,
                    byte_column: 1,
                },
                end: Position {
                    line: 0,
                    byte_column: 2,
                },
                text: "x".to_string(),
            }],
        };
        let error = app
            .handle_plugin_request("editor.apply_edit", serde_json::to_value(split).unwrap())
            .unwrap_err();
        assert_eq!(error.code, INVALID_POSITION, "{text}");
        assert_eq!(app.editor.text_lines(), vec![text], "{text}");
    }
}

#[test]
fn broken_manifest_warns_and_keeps_good_plugins() {
    let root = temp_dir("bad_manifest");
    manifest(&root);
    let broken = root.join("broken");
    fs::create_dir_all(&broken).unwrap();
    fs::write(broken.join("plugin.toml"), "name = 7\nprotocol = true\n").unwrap();
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.iter().any(|warning| warning.contains("broken")));
    assert!(app.commands.find("echo.ping").is_some());
    assert_eq!(app.keymaps.bindings().len(), 2);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn returns_method_not_found_for_unknown_host_methods() {
    let mut app = app();
    let error = app
        .handle_plugin_request("editor.unknown", json!({}))
        .unwrap_err();
    assert_eq!(error.code, kawi_plugin::METHOD_NOT_FOUND);
}

#[test]
fn plugin_context_carries_the_current_cursor() {
    let app = App::new(Buffer::from_string("abé"), Flavor::Vim);
    let context = app.plugin_context();
    assert_eq!(context.cursor.line, 0);
    assert_eq!(context.cursor.byte_column, 0);
    assert_eq!(context.mode, "normal");
}

#[test]
fn invokes_a_plugin_and_serves_host_requests() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let root = temp_dir("invoke");
    let dir = root.join("echo");
    fs::create_dir_all(&dir).unwrap();
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/echo");
    fs::copy(source.join("plugin.toml"), dir.join("plugin.toml")).unwrap();
    fs::copy(source.join("plugin.py"), dir.join("plugin.py")).unwrap();
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    let command = app.commands.find("echo.hello").expect("hello command");
    let target = match app.commands.spec(command).expect("spec").handler {
        CommandHandler::Plugin(target) => target,
        other => panic!("unexpected handler: {other:?}"),
    };
    app.run_plugin_command(target, None).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.message.as_deref() != Some("pong world") && Instant::now() < deadline {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(app.message.as_deref(), Some("pong world"));
    assert_eq!(app.plugins.status("echo"), Some(PluginStatus::Ready));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn trim_plugin_applies_a_real_edit() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let root = temp_dir("trim");
    let dir = root.join("trim");
    fs::create_dir_all(&dir).unwrap();
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/trim");
    fs::copy(source.join("plugin.toml"), dir.join("plugin.toml")).unwrap();
    fs::copy(source.join("plugin.py"), dir.join("plugin.py")).unwrap();
    let mut app = App::new(Buffer::from_string("alpha  \nbeta\t\n"), Flavor::Vim);
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    let command = app.commands.find("trim.trim").expect("trim command");
    let target = match app.commands.spec(command).expect("spec").handler {
        CommandHandler::Plugin(target) => target,
        other => panic!("unexpected handler: {other:?}"),
    };
    app.run_plugin_command(target, None).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.editor.document_snapshot() != "alpha\nbeta\n" && Instant::now() < deadline {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(app.editor.document_snapshot().to_string(), "alpha\nbeta\n");
    fs::remove_dir_all(root).unwrap();
}

fn event_manifest(root: &std::path::Path, mode: &str, debounce: Option<u64>) -> PathBuf {
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
            "name = \"events\"\nversion = \"0.1.0\"\nprotocol = 1\ncommand = [\"python3\", \"${{plugin_dir}}/event_plugin.py\", \"{mode}\"]\ntimeout_ms = 5000\nsubscribes = [\"buffer_changed\"]\n\n{debounce}[[activation_events]]\nevent = \"on_startup\"\n"
        ),
    )
    .unwrap();
    dir
}

fn save_buffer(app: &mut App) -> bool {
    app.request_save(false, crate::app::save::SaveAfter::None, false)
}

fn wait_for_plugin(app: &mut App, plugin: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.plugins.status(plugin) != Some(PluginStatus::Ready) && Instant::now() < deadline {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(app.plugins.status(plugin), Some(PluginStatus::Ready));
}

#[test]
fn plugin_edits_are_not_delivered_back_to_the_plugin() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let root = temp_dir("self_edit");
    let dir = event_manifest(&root, "edit", None);
    let mut app = App::new(Buffer::from_string("hello\n"), Flavor::Vim);
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    wait_for_plugin(&mut app, "events");
    app.editor.paste("x");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !app.editor.document_snapshot().to_string().contains('!') && Instant::now() < deadline {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    std::thread::sleep(Duration::from_millis(200));
    app.poll_plugins();
    assert_eq!(
        fs::read_to_string(dir.join("event-edit.count")).unwrap(),
        "1"
    );
    assert_eq!(app.editor.document_snapshot().to_string(), "hxello\n!");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn coalesces_buffer_changes_and_reports_a_resync() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let root = temp_dir("coalesce");
    let dir = event_manifest(&root, "log", Some(50));
    let mut app = App::new(Buffer::from_string("alpha\n"), Flavor::Vim);
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    wait_for_plugin(&mut app, "events");
    app.editor.paste("a");
    app.editor.paste("b");
    app.editor.paste("c");
    let log = dir.join("events.log");
    let deadline = Instant::now() + Duration::from_secs(5);
    while (!log.exists() || fs::read_to_string(&log).unwrap().lines().count() == 0)
        && Instant::now() < deadline
    {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    let text = fs::read_to_string(&log).unwrap();
    let events = text
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["full_resync"], true);
    assert_eq!(events[0]["changes"].as_array().unwrap().len(), 0);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn file_open_activation_trims_on_save() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let root = temp_dir("trim_event");
    let dir = root.join("trim");
    fs::create_dir_all(&dir).unwrap();
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/trim");
    fs::copy(source.join("plugin.toml"), dir.join("plugin.toml")).unwrap();
    fs::copy(source.join("plugin.py"), dir.join("plugin.py")).unwrap();
    let path = root.join("a.txt");
    fs::write(&path, "alpha  \n").unwrap();
    let mut app = App::new(
        Buffer::from_file(path.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    wait_for_plugin(&mut app, "trim");
    assert!(save_buffer(&mut app));
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.editor.document_snapshot() != "alpha\n" && Instant::now() < deadline {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(app.editor.document_snapshot().to_string(), "alpha\n");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn preserves_the_revision_chain_when_a_plugin_stalls() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let root = temp_dir("stall_chain");
    let dir = event_manifest(&root, "stall", None);
    let mut app = App::new(Buffer::from_string("alpha\n"), Flavor::Vim);
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    wait_for_plugin(&mut app, "events");
    for _ in 0..5 {
        app.editor.paste("x");
        app.poll_plugins();
    }
    let log = dir.join("events.log");
    let deadline = Instant::now() + Duration::from_secs(5);
    while (!log.exists() || fs::read_to_string(&log).unwrap().lines().count() < 5)
        && Instant::now() < deadline
    {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    let text = fs::read_to_string(&log).unwrap();
    let events = text
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(events.len(), 5);
    for (index, event) in events.iter().enumerate() {
        let revision = index as u64 + 1;
        assert_eq!(event["revision"], revision);
        assert_eq!(event["previous_revision"], revision.saturating_sub(1));
        assert_eq!(event["full_resync"], index == 0);
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn opening_many_files_spawns_once_and_preserves_order() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let root = temp_dir("many_files");
    let dir = root.join("opener");
    fs::create_dir_all(&dir).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/app/plugin/test_data/activation_plugin.py");
    fs::copy(source, dir.join("activation_plugin.py")).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        "name = \"opener\"\nversion = \"0.1.0\"\nprotocol = 1\ncommand = [\"python3\", \"${plugin_dir}/activation_plugin.py\"]\ntimeout_ms = 5000\nsubscribes = [\"file_opened\"]\n\n[[activation_events]]\nevent = \"on_file_open\"\n",
    )
    .unwrap();
    let files = (0..50)
        .map(|index| {
            let path = root.join(format!("f{index:02}.txt"));
            fs::write(&path, "text\n").unwrap();
            path
        })
        .collect::<Vec<_>>();
    let mut app = App::new(
        Buffer::from_file(files[0].to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    wait_for_plugin(&mut app, "opener");
    for path in files.iter().skip(1) {
        app.editor.open_file_buffer(path.to_str().unwrap()).unwrap();
    }
    app.poll_plugins();
    let log = dir.join("opened.log");
    let deadline = Instant::now() + Duration::from_secs(5);
    while (!log.exists() || fs::read_to_string(&log).unwrap().lines().count() < 50)
        && Instant::now() < deadline
    {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        fs::read_to_string(dir.join("spawns.log"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    let opened = fs::read_to_string(&log).unwrap();
    let expected = files
        .iter()
        .map(|path| path.to_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(opened.lines().collect::<Vec<_>>(), expected);
    fs::remove_dir_all(root).unwrap();
}
