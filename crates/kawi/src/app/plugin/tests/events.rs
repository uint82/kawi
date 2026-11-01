use std::fs;
use std::time::{Duration, Instant};

use kawi_buffer::Buffer;

use super::common::*;
use crate::app::App;
use crate::keymap::Flavor;

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
fn debounced_buffer_change_schedules_a_plugin_wait_timeout() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let root = temp_dir("debounce_wait");
    event_manifest(&root, "log", Some(50));
    let mut app = App::new(Buffer::from_string("alpha\n"), Flavor::Vim);
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    wait_for_plugin(&mut app, "events");
    app.editor.paste("a");
    app.poll_plugins();
    let timeout = app.plugin_wait_timeout().expect("plugin wait timeout");
    assert!(timeout <= Duration::from_millis(50));
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
        "name = \"opener\"\nversion = \"0.1.0\"\nprotocol = 1\ncommand = [\"python3\", \"${plugin_dir}/activation_plugin.py\"]\ntimeout_ms = 5000\nsubscribes = [\"file_opened\"]\n\n[capabilities]\n\"editor.read\" = true\n\n[[activation_events]]\nevent = \"on_file_open\"\n",
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
