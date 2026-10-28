use std::fs;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
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
    assert!(app
        .message
        .as_deref()
        .is_some_and(|message| message.contains("requests")));
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
    while !app.plugin_grants.is_active() && Instant::now() < deadline {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(app.plugin_grants.is_active());
    assert!(app
        .message
        .as_deref()
        .is_some_and(|message| message.contains("trim requests editor.read")));
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

#[test]
fn denial_opens_a_prompt_that_plugin_keys_cannot_answer() {
    let root = temp_dir("prompt_keys_send");
    denied_manifest(&root);
    let mut app = app();
    load_plugins_with_permissions(&mut app, &root, "");
    app.handle_plugin_host_request("denied", json!(null), "editor.get_state", json!({}));
    assert!(app.plugin_grants.is_active());
    app.handle_send_keys(Some(&toml::Value::String("<Enter>".to_string())))
        .unwrap();
    assert!(app.plugin_grants.is_active());
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    assert!(!app.plugin_grants.is_active());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn allow_once_grants_the_next_call() {
    let root = temp_dir("prompt_allow_once");
    denied_manifest(&root);
    let mut app = app();
    load_plugins_with_permissions(&mut app, &root, "");
    app.handle_plugin_host_request("denied", json!(null), "editor.get_state", json!({}));
    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))
        .unwrap();
    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))
        .unwrap();
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    assert!(!app.plugin_grants.is_active());
    app.handle_plugin_host_request("denied", json!(null), "editor.get_state", json!({}));
    assert!(!app.plugin_grants.is_active());
    assert_eq!(app.plugins.denial_records("denied").len(), 0);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn allow_always_writes_permissions_and_reloads() {
    let root = temp_dir("prompt_allow_always");
    denied_manifest(&root);
    let mut app = app();
    load_plugins_with_permissions(&mut app, &root, "");
    app.handle_plugin_host_request("denied", json!(null), "editor.get_state", json!({}));
    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))
        .unwrap();
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    assert!(!app.plugin_grants.is_active());
    assert!(app
        .plugins
        .has_capability("denied", kawi_plugin::Capability::EditorRead));
    let permissions = fs::read_to_string(root.join("permissions.toml")).unwrap();
    assert!(permissions.contains("\"editor.read\" = true"));
    assert!(permissions.contains("request_hash"));
    assert!(app
        .message
        .as_deref()
        .is_some_and(|message| message.contains("Granted editor.read")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_grant_write_falls_back_to_allow_once() {
    let root = temp_dir("prompt_write_failure");
    denied_manifest(&root);
    let mut app = app();
    load_plugins_with_permissions(&mut app, &root, "");
    let permissions = root.join("permissions.toml");
    fs::remove_file(&permissions).unwrap();
    fs::create_dir(&permissions).unwrap();
    app.handle_plugin_host_request("denied", json!(null), "editor.get_state", json!({}));
    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))
        .unwrap();
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    assert!(
        app.message
            .as_deref()
            .is_some_and(|message| message.contains("allowing once")),
        "{:?}",
        app.message
    );
    assert!(app
        .plugins
        .take_allow_next("denied", kawi_plugin::Capability::EditorRead));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn typed_grant_command_opens_the_same_prompt() {
    let root = temp_dir("prompt_typed_grant");
    denied_manifest(&root);
    let mut app = app();
    load_plugins_with_permissions(&mut app, &root, "");
    app.handle_plugin_command("grant denied editor.read");
    assert!(app.plugin_grants.is_active());
    assert!(app
        .message
        .as_deref()
        .is_some_and(|message| message.contains("denied requests editor.read")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn hash_change_prompt_lists_added_requests() {
    let root = temp_dir("prompt_hash_diff");
    denied_manifest(&root);
    let mut app = app();
    load_plugins_with_permissions(
        &mut app,
        &root,
        "[plugins.denied]\nrequest_hash = \"sha256:stale\"\n\n[plugins.denied.grant]\n\"editor.read\" = true\n",
    );
    app.handle_plugin_host_request("denied", json!(null), "editor.apply_edit", json!({}));
    assert!(
        app.message
            .as_deref()
            .is_some_and(|message| message.contains("editor.write added")),
        "{:?}",
        app.message
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn permissions_file_changes_reload_grants() {
    let root = temp_dir("permissions_watch");
    denied_manifest(&root);
    let mut app = app();
    load_plugins_with_permissions(&mut app, &root, "");
    assert!(!app
        .plugins
        .has_capability("denied", kawi_plugin::Capability::EditorRead));
    std::thread::sleep(Duration::from_millis(150));
    fs::write(
        root.join("permissions.toml"),
        "[plugins.denied.grant]\n\"editor.read\" = true\n",
    )
    .unwrap();
    wait_for_grant(&mut app, true);
    assert!(app
        .plugins
        .has_capability("denied", kawi_plugin::Capability::EditorRead));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn malformed_permission_saves_fail_closed() {
    let root = temp_dir("permissions_watch_invalid");
    denied_manifest(&root);
    let mut app = app();
    load_plugins_with_permissions(
        &mut app,
        &root,
        "[plugins.denied.grant]\n\"editor.read\" = true\n",
    );
    assert!(app
        .plugins
        .has_capability("denied", kawi_plugin::Capability::EditorRead));
    std::thread::sleep(Duration::from_millis(150));
    fs::write(root.join("permissions.toml"), "not toml").unwrap();
    wait_for_grant(&mut app, false);
    assert!(!app
        .plugins
        .has_capability("denied", kawi_plugin::Capability::EditorRead));
    assert!(app
        .plugins
        .capability_lines("denied")
        .iter()
        .any(|line| line.contains("ungranted")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn grant_writes_do_not_trigger_a_reload_loop() {
    let root = temp_dir("permissions_watch_own_write");
    denied_manifest(&root);
    let mut app = app();
    load_plugins_with_permissions(&mut app, &root, "");
    app.handle_plugin_host_request("denied", json!(null), "editor.get_state", json!({}));
    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))
        .unwrap();
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    std::thread::sleep(Duration::from_millis(200));
    app.poll_plugins();
    assert!(app
        .message
        .as_deref()
        .is_some_and(|message| message.contains("Granted editor.read")));
    fs::remove_dir_all(root).unwrap();
}

fn wait_for_grant(app: &mut App, granted: bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while app
        .plugins
        .has_capability("denied", kawi_plugin::Capability::EditorRead)
        != granted
        && Instant::now() < deadline
    {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn plugin_messages_cannot_overwrite_a_grant_prompt() {
    let root = temp_dir("prompt_spoof");
    denied_manifest(&root);
    let mut app = app();
    load_plugins_with_permissions(&mut app, &root, "");
    app.handle_plugin_host_request("denied", json!(null), "editor.get_state", json!({}));
    app.handle_plugin_host_request(
        "denied",
        json!(null),
        "ui.message",
        json!({"message": "spoofed consent"}),
    );
    assert!(app.plugin_grants.is_active());
    assert!(app
        .message
        .as_deref()
        .is_some_and(|message| message.contains("denied requests editor.read")));
    assert!(!app
        .message
        .as_deref()
        .is_some_and(|message| message.contains("spoofed")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[cfg(target_os = "linux")]
fn sandboxed_plugins_are_labeled_enforced_after_spawn() {
    if !kawi_process::sandbox_supported() {
        return;
    }
    let root = temp_dir("sandbox_label");
    let dir = root.join("sandboxed");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        r#"name = "sandboxed"
version = "0.1.0"
protocol = 1
command = ["/bin/true"]

[capabilities]
"fs.read" = { paths = ["$WORKSPACE"] }

[[commands]]
id = "ping"
title = "Ping"
"#,
    )
    .unwrap();
    let mut app = app();
    load_plugins_with_permissions(
        &mut app,
        &root,
        "[plugins.sandboxed.grant]\n\"fs.read\" = { paths = [\"$WORKSPACE\"] }\n",
    );
    let command = app.commands.find("sandboxed.ping").expect("command");
    let target = match app.commands.spec(command).expect("spec").handler {
        CommandHandler::Plugin(target) => target,
        other => panic!("unexpected handler: {other:?}"),
    };
    app.run_plugin_command(target, None).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        app.poll_plugins();
        if app
            .plugins
            .capability_lines("sandboxed")
            .iter()
            .any(|line| line.contains("fs.read (enforced)"))
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(app
        .plugins
        .capability_lines("sandboxed")
        .iter()
        .any(|line| line.contains("fs.read (enforced)")));
    fs::remove_dir_all(root).unwrap();
}
