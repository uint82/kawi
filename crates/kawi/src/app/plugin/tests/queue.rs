use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use kawi_editor::Action;
use kawi_plugin::PluginStatus;
use toml::value::Table;

use super::common::*;
use crate::app::App;
use crate::command::CommandHandler;

mod gates;
mod visibility;

fn queue_plugin(root: &Path) {
    let dir = root.join("queue");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        r#"name = "queue"
version = "0.1.0"
protocol = 1
command = ["python3", "${plugin_dir}/queue_plugin.py"]
timeout_ms = 5000

[capabilities]
"editor.read" = true
"editor.write" = true
"ui.message" = true

[[commands]]
id = "report"
title = "Report"
capabilities = ["editor.write"]
coalesce_queued = true

[[commands]]
id = "plain"
title = "Plain"
"#,
    )
    .unwrap();
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/app/plugin/test_data/queue_plugin.py");
    fs::copy(source, dir.join("queue_plugin.py")).unwrap();
}

fn setup(name: &str) -> (PathBuf, App, u32) {
    let root = temp_dir(name);
    queue_plugin(&root);
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    let target = target(&app, "queue.report");
    (root, app, target)
}

fn target(app: &App, name: &str) -> u32 {
    let command = app.commands.find(name).expect("plugin command");
    match app.commands.spec(command).expect("spec").handler {
        CommandHandler::Plugin(target) => target,
        other => panic!("unexpected handler: {other:?}"),
    }
}

fn args(sleep_ms: u64, report: bool) -> Option<toml::Value> {
    let mut table = Table::new();
    table.insert(
        "sleep_ms".to_string(),
        toml::Value::Integer(sleep_ms as i64),
    );
    table.insert("report".to_string(), toml::Value::Boolean(report));
    Some(toml::Value::Table(table))
}

fn crash_args(sleep_ms: u64) -> Option<toml::Value> {
    let mut table = Table::new();
    table.insert(
        "sleep_ms".to_string(),
        toml::Value::Integer(sleep_ms as i64),
    );
    table.insert("crash".to_string(), toml::Value::Boolean(true));
    Some(toml::Value::Table(table))
}

fn poll_until(app: &mut App, timeout: Duration, predicate: impl Fn(&App) -> bool) {
    let deadline = Instant::now() + timeout;
    while !predicate(app) && Instant::now() < deadline {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn queued_command_runs_with_fresh_context() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let (root, mut app, target) = setup("queue_fresh");
    let first_revision = app.editor.buffer_revision();
    app.run_plugin_command(target, args(1200, false)).unwrap();
    app.run_plugin_command(target, args(0, true)).unwrap();
    assert_eq!(app.plugins.queue_depth("queue"), 1);
    assert!(app
        .message
        .as_deref()
        .is_some_and(|message| message.contains("busy, 1 queued")));
    app.editor.apply(Action::EnterInsert);
    app.editor.apply(Action::InsertChar('x'));
    assert_eq!(app.editor.buffer_revision(), first_revision + 1);
    poll_until(&mut app, Duration::from_secs(5), |app| {
        app.message
            .as_deref()
            .is_some_and(|message| message.starts_with("revision:"))
    });
    assert_eq!(
        app.message.as_deref(),
        Some(format!("revision:{}", first_revision + 1).as_str())
    );
    assert_eq!(app.plugins.queue_depth("queue"), 0);
    app.plugins.shutdown();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn full_queue_rejects_new_commands() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let (root, mut app, target) = setup("queue_full");
    app.run_plugin_command(target, args(1200, false)).unwrap();
    for index in 0..4 {
        app.run_plugin_command(target, args(index + 1, false))
            .unwrap();
    }
    app.run_plugin_command(target, args(0, false)).unwrap();
    assert_eq!(app.plugins.queue_depth("queue"), 4);
    assert_eq!(app.plugins.queue_counters("queue").rejected, 1);
    assert!(app
        .message
        .as_deref()
        .is_some_and(|message| message.contains("queue is full")));
    app.plugins.shutdown();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn identical_queued_commands_coalesce() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let (root, mut app, target) = setup("queue_coalesce");
    app.run_plugin_command(target, args(1200, false)).unwrap();
    app.run_plugin_command(target, args(0, true)).unwrap();
    app.run_plugin_command(target, args(0, true)).unwrap();
    assert_eq!(app.plugins.queue_depth("queue"), 1);
    app.plugins.shutdown();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn revoked_command_is_denied_at_dequeue() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let (root, mut app, target) = setup("queue_revoke");
    app.run_plugin_command(target, args(1200, false)).unwrap();
    app.run_plugin_command(target, args(0, true)).unwrap();
    write_permissions(&root, "[plugins.queue.grant]\n\"ui.message\" = true\n");
    app.plugins.reload_permissions();
    poll_until(&mut app, Duration::from_secs(5), |app| {
        app.plugins
            .denial_records("queue")
            .iter()
            .any(|line| line.contains("editor.write"))
    });
    assert!(app
        .plugins
        .logs("queue")
        .iter()
        .any(|line| line.contains("denied editor.write")));
    assert!(!app
        .message
        .as_deref()
        .is_some_and(|message| message.starts_with("revision:")));
    assert_eq!(app.plugins.queue_depth("queue"), 0);
    app.plugins.shutdown();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn expired_queued_command_is_dropped() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let (root, mut app, target) = setup("queue_expiry");
    app.plugins.queue_wait_override = Some(Duration::from_millis(100));
    app.run_plugin_command(target, args(1200, false)).unwrap();
    app.run_plugin_command(target, args(0, true)).unwrap();
    assert_eq!(app.plugins.queue_depth("queue"), 1);
    poll_until(&mut app, Duration::from_secs(2), |app| {
        app.plugins.queue_depth("queue") == 0
    });
    assert!(app
        .plugins
        .logs("queue")
        .iter()
        .any(|line| line.contains("(queue wait exceeded)")));
    app.plugins.shutdown();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn timed_out_command_drops_queued_commands() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let root = temp_dir("queue_timeout");
    queue_plugin(&root);
    let manifest = root.join("queue/plugin.toml");
    let text = fs::read_to_string(&manifest)
        .unwrap()
        .replace("timeout_ms = 5000", "timeout_ms = 200");
    fs::write(&manifest, text).unwrap();
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    let target = target(&app, "queue.report");
    app.run_plugin_command(target, args(1200, false)).unwrap();
    app.run_plugin_command(target, args(0, true)).unwrap();
    assert_eq!(app.plugins.queue_depth("queue"), 1);
    poll_until(&mut app, Duration::from_secs(5), |app| {
        app.plugins.queue_depth("queue") == 0
    });
    assert!(app
        .plugins
        .logs("queue")
        .iter()
        .any(|line| line.starts_with("queued command dropped:")));
    assert!(!app
        .message
        .as_deref()
        .is_some_and(|message| message.starts_with("revision:")));
    app.plugins.shutdown();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn crashed_plugin_drops_queued_commands() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let (root, mut app, target) = setup("queue_crash");
    app.run_plugin_command(target, crash_args(600)).unwrap();
    app.run_plugin_command(target, args(0, true)).unwrap();
    assert_eq!(app.plugins.queue_depth("queue"), 1);
    poll_until(&mut app, Duration::from_secs(5), |app| {
        app.plugins.status("queue") == Some(PluginStatus::Backoff { remaining_ms: 0 })
    });
    assert_eq!(app.plugins.queue_depth("queue"), 0);
    assert!(app
        .plugins
        .logs("queue")
        .iter()
        .any(|line| line.starts_with("queued command dropped:")));
    assert!(!app
        .message
        .as_deref()
        .is_some_and(|message| message.starts_with("revision:")));
    app.plugins.shutdown();
    fs::remove_dir_all(root).unwrap();
}
