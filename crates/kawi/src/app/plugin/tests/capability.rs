use std::fs;
use std::time::{Duration, Instant};

use kawi_buffer::Buffer;
use kawi_plugin::{PluginStatus, CAPABILITY_DENIED, HOST_METHODS};
use serde_json::json;

use super::common::*;
use crate::app::App;
use crate::command::CommandHandler;
use crate::keymap::Flavor;

#[test]
fn every_host_method_is_denied_without_a_grant() {
    let root = temp_dir("deny_methods");
    denied_manifest(&root);
    let mut app = app();
    load_plugins_with_permissions(
        &mut app,
        &root,
        "[plugins.denied.grant]\n\"ui.message\" = false\n",
    );
    for method in HOST_METHODS {
        let error = denied_error(&app, method);
        assert_eq!(error.code, CAPABILITY_DENIED, "{method}");
        assert_eq!(
            error.data.as_ref().unwrap()["capability"],
            method_capability(method)
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn denied_notifications_are_logged_without_a_response() {
    let root = temp_dir("deny_notification");
    denied_manifest(&root);
    let mut app = app();
    load_plugins_with_permissions(
        &mut app,
        &root,
        "[plugins.denied.grant]\n\"ui.message\" = false\n",
    );
    for method in HOST_METHODS {
        app.handle_plugin_host_request("denied", json!(null), method, json!({}));
    }
    let logs = app.plugins.logs("denied");
    assert!(logs.iter().any(|line| line == "denied editor.read"));
    assert!(logs.iter().any(|line| line == "denied editor.write"));
    assert!(logs.iter().any(|line| line == "denied ui.message"));
    assert!(app.message.is_none());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn denial_records_are_deduplicated_with_a_counter() {
    let root = temp_dir("deny_dedupe");
    denied_manifest(&root);
    let mut app = app();
    load_plugins_with_permissions(
        &mut app,
        &root,
        "[plugins.denied.grant]\n\"ui.message\" = false\n",
    );
    app.handle_plugin_host_request("denied", json!(null), "editor.get_state", json!({}));
    app.handle_plugin_host_request("denied", json!(null), "editor.get_state", json!({}));
    let logs = app.plugins.logs("denied");
    assert_eq!(
        logs.iter()
            .filter(|line| *line == "denied editor.read")
            .count(),
        1
    );
    assert!(logs
        .iter()
        .any(|line| line == "denied 2 calls: editor.read"));
    assert!(logs
        .iter()
        .any(|line| line == "add to permissions.toml: [plugins.denied.grant]"));
    assert!(logs.iter().any(|line| line == "\"editor.read\" = true"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ui_messages_are_rate_limited_per_plugin() {
    let root = temp_dir("message_limit");
    denied_manifest(&root);
    let mut app = app();
    load_plugins_with_permissions(&mut app, &root, "");
    app.handle_plugin_host_request(
        "denied",
        json!(null),
        "ui.message",
        json!({"message": "first"}),
    );
    app.handle_plugin_host_request(
        "denied",
        json!(null),
        "ui.message",
        json!({"message": "second"}),
    );
    assert_eq!(app.message.as_deref(), Some("first"));
    assert!(app
        .plugins
        .logs("denied")
        .iter()
        .any(|line| line == "ui.message rate limited"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unprivileged_plugins_are_listed_without_spawning() {
    let root = temp_dir("list_unprivileged");
    manifest(&root);
    let mut app = app();
    load_plugins_with_permissions(&mut app, &root, "");
    assert_eq!(app.plugins.status("echo"), Some(PluginStatus::Stopped));
    assert_eq!(app.plugins.capability_lines("echo"), vec!["granted: none"]);
    app.handle_plugin_command("list");
    let lines = app.pager.as_ref().expect("pager");
    assert!(lines[0].contains("echo"));
    assert!(lines[0].contains("stopped"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn denied_edit_returns_capability_error_before_stale_revision() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let root = temp_dir("probe_stale");
    let dir = root.join("probe");
    fs::create_dir_all(&dir).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/app/plugin/test_data/stale_probe.py");
    fs::copy(source, dir.join("stale_probe.py")).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        r#"name = "probe"
version = "0.1.0"
protocol = 1
command = ["python3", "${plugin_dir}/stale_probe.py"]
timeout_ms = 5000

[capabilities]
"editor.write" = true

[[commands]]
id = "probe"
title = "Probe"
"#,
    )
    .unwrap();
    let mut app = App::new(Buffer::from_string("text\n"), Flavor::Vim);
    load_plugins_with_permissions(&mut app, &root, "");
    let command = app.commands.find("probe.probe").expect("probe command");
    let target = match app.commands.spec(command).expect("spec").handler {
        CommandHandler::Plugin(target) => target,
        other => panic!("unexpected handler: {other:?}"),
    };
    app.run_plugin_command(target, None).unwrap();
    let log = dir.join("probe.log");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !log.exists() && Instant::now() < deadline {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    let response: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&log).unwrap()).unwrap();
    assert_eq!(response["error"]["code"], CAPABILITY_DENIED);
    assert_eq!(response["error"]["data"]["capability"], "editor.write");
    fs::remove_dir_all(root).unwrap();
}
