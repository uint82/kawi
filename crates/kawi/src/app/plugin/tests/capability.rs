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
    app.handle_plugin_command("list");
    let list = app.pager.as_ref().expect("pager");
    assert!(list
        .iter()
        .any(|line| line.contains("add to permissions.toml: [plugins.denied.grant]")));
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
        .any(|line| line == "ui.message rate limited: 1 dropped"));
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

#[test]
fn revocation_applies_to_an_in_flight_invocation() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let root = temp_dir("inflight_revocation");
    let dir = root.join("inflight");
    fs::create_dir_all(&dir).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/app/plugin/test_data/inflight_probe.py");
    fs::copy(source, dir.join("inflight_probe.py")).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        r#"name = "inflight"
version = "0.1.0"
protocol = 1
command = ["python3", "${plugin_dir}/inflight_probe.py"]
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
    load_plugins_with_permissions(
        &mut app,
        &root,
        "[plugins.inflight.grant]\n\"editor.write\" = true\n",
    );
    let command = app.commands.find("inflight.probe").expect("probe command");
    let target = match app.commands.spec(command).expect("spec").handler {
        CommandHandler::Plugin(target) => target,
        other => panic!("unexpected handler: {other:?}"),
    };
    app.run_plugin_command(target, None).unwrap();
    let ready = dir.join("inflight.ready");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !ready.exists() && Instant::now() < deadline {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(ready.exists());
    fs::write(root.join("permissions.toml"), "").unwrap();
    app.plugins.reload_permissions();
    fs::write(dir.join("inflight.go"), "go").unwrap();
    let log = dir.join("inflight.log");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !log.exists() && Instant::now() < deadline {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    let response: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&log).unwrap()).unwrap();
    assert_eq!(response["error"]["code"], CAPABILITY_DENIED);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn malformed_reload_fails_closed_to_the_default_tier() {
    let root = temp_dir("failed_reload");
    denied_manifest(&root);
    let mut app = app();
    load_plugins_with_permissions(
        &mut app,
        &root,
        "[plugins.denied.grant]\n\"editor.read\" = true\n\"editor.write\" = true\n",
    );
    assert!(app
        .plugins
        .denied_capability("denied", "editor.write")
        .is_none());
    fs::write(root.join("permissions.toml"), "not toml").unwrap();
    let warnings = app.plugins.reload_permissions();
    assert!(warnings
        .iter()
        .any(|warning| warning.contains("permissions.toml")));
    assert_eq!(
        app.plugins.denied_capability("denied", "editor.apply_edit"),
        Some(kawi_plugin::Capability::EditorWrite)
    );
    assert!(app
        .plugins
        .denied_capability("denied", "ui.message")
        .is_none());
    assert!(app
        .plugins
        .capability_lines("denied")
        .iter()
        .any(|line| line.contains("ungranted: editor.read, editor.write")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn stale_request_hash_shows_ungranted_and_changed() {
    let root = temp_dir("stale_hash");
    denied_manifest(&root);
    let mut app = app();
    load_plugins_with_permissions(
        &mut app,
        &root,
        "[plugins.denied]\nrequest_hash = \"sha256:stale\"\n\n[plugins.denied.grant]\n\"editor.read\" = true\n\"editor.write\" = true\n",
    );
    let lines = app.plugins.capability_lines("denied");
    assert!(lines
        .iter()
        .any(|line| line == "changed: request hash mismatch"));
    assert!(lines
        .iter()
        .any(|line| line == "ungranted: editor.read, editor.write"));
    app.handle_plugin_command("list");
    let list = app.pager.as_ref().expect("pager");
    assert!(list
        .iter()
        .any(|line| line.contains("changed: request hash mismatch")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn example_plugin_reports_capability_denial_without_grants() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let root = temp_dir("example_denial");
    let dir = root.join("trim");
    fs::create_dir_all(&dir).unwrap();
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/trim");
    fs::copy(source.join("plugin.toml"), dir.join("plugin.toml")).unwrap();
    fs::copy(source.join("plugin.py"), dir.join("plugin.py")).unwrap();
    let mut app = App::new(Buffer::from_string("alpha  \n"), Flavor::Vim);
    load_plugins_with_permissions(&mut app, &root, "");
    let command = app.commands.find("trim.trim").expect("trim command");
    let target = match app.commands.spec(command).expect("spec").handler {
        CommandHandler::Plugin(target) => target,
        other => panic!("unexpected handler: {other:?}"),
    };
    app.run_plugin_command(target, None).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.message.as_deref() != Some("capability denied: editor.read")
        && Instant::now() < deadline
    {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        app.message.as_deref(),
        Some("capability denied: editor.read")
    );
    app.handle_plugin_command("list");
    let list = app.pager.as_ref().expect("pager");
    assert!(list.iter().any(|line| line.contains("denied editor.read")));
    assert!(list
        .iter()
        .any(|line| line.contains("add to permissions.toml: [plugins.trim.grant]")));
    assert!(list
        .iter()
        .any(|line| line.contains("\"editor.read\" = true")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn denial_records_are_bounded_to_known_plugins() {
    let root = temp_dir("deny_bounded");
    denied_manifest(&root);
    let mut app = app();
    load_plugins_with_permissions(
        &mut app,
        &root,
        "[plugins.denied.grant]\n\"ui.message\" = false\n",
    );
    app.handle_plugin_host_request("denied", json!(null), "editor.get_state", json!({}));
    app.plugins
        .record_denial("missing", kawi_plugin::Capability::EditorRead);
    assert!(app.plugins.logs("missing").is_empty());
    let empty = temp_dir("deny_bounded_empty");
    load_plugins_with_permissions(&mut app, &empty, "");
    assert!(app.plugins.logs("denied").is_empty());
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(empty).unwrap();
}

#[test]
fn advisory_capabilities_are_labeled_in_the_plugin_list() {
    let root = temp_dir("advisory_label");
    let dir = root.join("advisory");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        r#"name = "advisory"
version = "0.1.0"
protocol = 1
command = ["/bin/true"]

[capabilities]
"fs.read" = { paths = ["$WORKSPACE/src"] }
"#,
    )
    .unwrap();
    let mut app = app();
    load_plugins_with_permissions(
        &mut app,
        &root,
        "[plugins.advisory.grant]\n\"fs.read\" = { paths = [\"$WORKSPACE/src\"] }\n",
    );
    let lines = app.plugins.capability_lines("advisory");
    assert!(lines.iter().any(|line| line.contains("fs.read (advisory)")));
    fs::remove_dir_all(root).unwrap();
}
