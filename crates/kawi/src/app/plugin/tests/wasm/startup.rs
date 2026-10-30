use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use crate::app::App;
use crate::command::CommandHandler;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_plugin::{Capability, PluginError, PluginHostEvent};

use super::*;

fn startup_extra(command: &str) -> String {
    format!(
        "\n[[commands]]\nid = \"{command}\"\ntitle = \"Run\"\n\n[[activation_events]]\nevent = \"on_startup\"\n"
    )
}

fn write_fixture_plugin(root: &Path, fixture: &str, capabilities: &[&str], extra: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../kawi-plugin/tests/fixtures/wasm")
        .join(fixture);
    let bytes = fs::read(path).unwrap();
    write_plugin_manifest_with(root, &bytes, None, capabilities, extra)
}

fn write_startup_plugin(root: &Path) -> String {
    write_fixture_plugin(
        root,
        "editor-host.wasm",
        &["editor.read"],
        &startup_extra("read"),
    )
}

fn wait_for_grant_prompt(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !app.plugin_grants.is_active() && Instant::now() < deadline {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(app.plugin_grants.is_active());
}

fn allow_always(app: &mut App) {
    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))
        .unwrap();
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
}

fn wait_for_batch_prompt(app: &mut App, count: usize) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.plugin_grants.active_capabilities().len() < count && Instant::now() < deadline {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(app.plugin_grants.active_capabilities().len(), count);
}

fn allow_all(app: &mut App) {
    app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE))
        .unwrap();
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
}

fn wait_for_ready(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.plugins.status("wasm") != Some(PluginStatus::Ready) && Instant::now() < deadline {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(app.plugins.status("wasm"), Some(PluginStatus::Ready));
}

fn wait_for_start_failure(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        app.poll_plugins();
        if matches!(
            app.plugins.status("wasm"),
            Some(PluginStatus::Backoff { .. } | PluginStatus::Disabled)
        ) {
            app.poll_plugins();
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("wasm plugin did not report a startup failure");
}

fn wait_for_log_growth(app: &mut App, plugin: &str, previous: usize) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.plugins.logs(plugin).len() <= previous {
        assert!(Instant::now() < deadline, "plugin log did not grow");
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn plugin_target(app: &App, name: &str) -> u32 {
    let command = app.commands.find(name).expect("plugin command");
    match app.commands.spec(command).expect("command spec").handler {
        CommandHandler::Plugin(target) => target,
        other => panic!("unexpected handler: {other:?}"),
    }
}

fn wait_for_invocation(app: &mut App) -> Result<(), PluginError> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        for event in app.plugins.drain_events() {
            if let PluginHostEvent::InvocationFinished { result, .. } = event {
                return result.map(|_| ());
            }
        }
        assert!(Instant::now() < deadline, "plugin invocation timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn assert_grant_file(root: &Path, capabilities: &[Capability]) {
    let text = fs::read_to_string(root.join("permissions.toml")).unwrap();
    assert!(text.contains("request_hash"));
    for capability in capabilities {
        assert!(
            text.contains(&format!("\"{}\" = true", capability.name())),
            "{text}"
        );
    }
    assert!(fs::read_dir(root)
        .unwrap()
        .flatten()
        .all(|entry| { !entry.file_name().to_string_lossy().ends_with(".tmp") }));
}

#[test]
fn startup_import_denial_opens_grant_prompt_and_restarts_on_grant() {
    let root = temp_dir("wasm-startup-consent");
    write_startup_plugin(&root);
    let mut app = app();
    load_plugins_with_permissions(&mut app, &root, "");
    wait_for_grant_prompt(&mut app);
    assert!(app
        .message
        .as_deref()
        .is_some_and(|message| message.contains("wasm requests editor.read")));
    app.handle_send_keys(Some(&toml::Value::String("<Enter>".to_string())))
        .unwrap();
    assert!(app.plugin_grants.is_active());
    allow_always(&mut app);
    assert!(!app.plugin_grants.is_active());
    assert!(app
        .plugins
        .has_capability("wasm", kawi_plugin::Capability::EditorRead));
    wait_for_ready(&mut app);
    assert_grant_file(&root, &[Capability::EditorRead]);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn denied_startup_prompt_stays_quiet_until_the_request_changes() {
    let root = temp_dir("wasm-startup-denied");
    write_fixture_plugin(
        &root,
        "editor-host.wasm",
        &["editor.read", "editor.write"],
        &startup_extra("read"),
    );
    let mut app = app();
    load_plugins_with_permissions(
        &mut app,
        &root,
        "[plugins.wasm.grant]\n\"editor.write\" = true\n",
    );
    wait_for_grant_prompt(&mut app);
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        .unwrap();
    assert!(!app.plugin_grants.is_active());
    app.handle_plugin_command("list");
    let lines = app.pager.as_ref().expect("pager");
    assert!(lines.iter().any(|line| line.contains("denied editor.read")));
    assert!(lines
        .iter()
        .any(|line| line.contains("add to permissions.toml")));
    app.plugins.restart(Some("wasm")).unwrap();
    wait_for_start_failure(&mut app);
    assert!(!app.plugin_grants.is_active());
    let logs = app.plugins.logs("wasm").len();
    write_permissions(&root, "[plugins.wasm.grant]\n\"editor.write\" = false\n");
    app.plugins.reload_permissions();
    wait_for_log_growth(&mut app, "wasm", logs);
    assert!(!app.plugin_grants.is_active());
    let logs = app.plugins.logs("wasm").len();
    write_permissions(&root, "[plugins.wasm.grant]\n\"editor.write\" = true\n");
    wait_for_log_growth(&mut app, "wasm", logs);
    assert!(!app.plugin_grants.is_active());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn changed_request_prompts_again_after_a_denial() {
    let root = temp_dir("wasm-startup-request-change");
    write_startup_plugin(&root);
    let mut app = app();
    load_plugins_with_permissions(&mut app, &root, "");
    wait_for_grant_prompt(&mut app);
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        .unwrap();
    assert!(!app.plugin_grants.is_active());
    write_fixture_plugin(
        &root,
        "editor-host.wasm",
        &["editor.read", "editor.write"],
        &startup_extra("changed"),
    );
    load_plugins_with_permissions(&mut app, &root, "");
    wait_for_grant_prompt(&mut app);
    assert!(app
        .message
        .as_deref()
        .is_some_and(|message| message.contains("wasm requests editor.read")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_startup_grant_write_reports_an_error() {
    let root = temp_dir("wasm-startup-write-failure");
    write_startup_plugin(&root);
    let mut app = app();
    load_plugins_with_permissions(&mut app, &root, "");
    wait_for_grant_prompt(&mut app);
    let permissions = root.join("permissions.toml");
    fs::remove_file(&permissions).unwrap();
    fs::create_dir(&permissions).unwrap();
    allow_always(&mut app);
    assert!(!app.plugin_grants.is_active());
    assert!(!app.plugins.has_capability("wasm", Capability::EditorRead));
    assert!(app
        .message
        .as_deref()
        .is_some_and(|message| message.contains("cannot save grant")));
    assert!(!app
        .message
        .as_deref()
        .is_some_and(|message| message.contains("Granted")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn multiple_missing_imports_are_batched_and_invoked() {
    let root = temp_dir("wasm-startup-multiple");
    write_fixture_plugin(
        &root,
        "multi-import.wasm",
        &["editor.read", "ui.message", "diagnostics.publish"],
        &startup_extra("ping"),
    );
    let mut app = app();
    load_plugins_with_permissions(
        &mut app,
        &root,
        "[plugins.wasm.grant]\n\"ui.message\" = false\n",
    );
    let capabilities = [
        Capability::EditorRead,
        Capability::UiMessage,
        Capability::DiagnosticsPublish,
    ];
    wait_for_batch_prompt(&mut app, capabilities.len());
    let seen = app
        .plugin_grants
        .active_capabilities()
        .into_iter()
        .collect::<BTreeSet<_>>();
    assert_eq!(seen, BTreeSet::from(capabilities));
    allow_all(&mut app);
    assert!(!app.plugin_grants.is_active());
    wait_for_ready(&mut app);
    assert_grant_file(&root, &capabilities);
    let target = plugin_target(&app, "wasm.ping");
    app.run_plugin_command(target, None).unwrap();
    wait_for_invocation(&mut app).expect("plugin invocation");
    fs::remove_dir_all(root).unwrap();
}
