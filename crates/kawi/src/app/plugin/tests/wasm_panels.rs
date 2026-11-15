use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use sha2::{Digest, Sha256};

use super::common::{app, invoke_plugin, load_plugins, temp_dir};
use crate::app::App;
use crate::command::CommandHandler;

const PLUGIN: &str = "wasm-panels";
const MESSAGE_INTERVAL: Duration = Duration::from_millis(1100);

fn write_plugin(root: &Path) {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../kawi-plugin/tests/fixtures/wasm/panel-plugin.wasm");
    let bytes = fs::read(fixture).expect("panel fixture");
    let dir = root.join(PLUGIN);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("plugin.wasm"), &bytes).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        format!(
            r#"name = "{PLUGIN}"
version = "0.1.0"
protocol = 1
runtime = "wasm"
component = "plugin.wasm"
sha256 = "sha256:{:x}"
timeout_ms = 5000

[capabilities]
"ui.message" = true
"ui.panel" = true
"ui.overlay" = true
"ui.picker" = true

[[commands]]
id = "open"
title = "Open panel"

[[commands]]
id = "update"
title = "Update panel"

[[commands]]
id = "focus"
title = "Focus panel"

[[commands]]
id = "close"
title = "Close panel"

[[commands]]
id = "overlay-open"
title = "Open overlay"

[[commands]]
id = "overlay-update"
title = "Update overlay"

[[commands]]
id = "overlay-wait"
title = "Wait for overlay event"

[[commands]]
id = "overlay-close"
title = "Close overlay"

[[commands]]
id = "picker"
title = "Pick an item"
"#,
            Sha256::digest(&bytes)
        ),
    )
    .unwrap();
}

#[test]
fn wasm_plugin_controls_a_panel() {
    let root = temp_dir("wasm-panels");
    write_plugin(&root);
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");

    let opened = invoke_plugin(&mut app, &format!("{PLUGIN}.open")).unwrap_or_else(|| {
        panic!("open message: {:?}", app.plugins.logs(PLUGIN));
    });
    assert!(opened.starts_with("opened:"), "{opened}");
    let view = app.plugin_panel_view().expect("panel view");
    assert_eq!(view.title, "Index");
    assert_eq!(view.status, "building");
    assert_eq!(view.lines.len(), 2);
    assert!(!view.focused);

    std::thread::sleep(MESSAGE_INTERVAL);
    assert_eq!(
        invoke_plugin(&mut app, &format!("{PLUGIN}.update")),
        Some("updated".to_string()),
        "{:?}",
        app.plugins.logs(PLUGIN)
    );
    let view = app.plugin_panel_view().expect("panel view");
    assert_eq!(view.status, "complete");
    assert_eq!(view.lines[0].text, "gamma");
    assert_eq!(app.plugin_panel_rect().unwrap().height, 5);

    std::thread::sleep(MESSAGE_INTERVAL);
    assert_eq!(
        invoke_plugin(&mut app, &format!("{PLUGIN}.focus")),
        Some("focused".to_string())
    );
    assert!(app.plugin_panel_view().expect("panel view").focused);

    std::thread::sleep(MESSAGE_INTERVAL);
    assert_eq!(
        invoke_plugin(&mut app, &format!("{PLUGIN}.close")),
        Some("closed".to_string())
    );
    assert!(app.plugin_panel_view().is_none());
    fs::remove_dir_all(root).unwrap();
}

fn start_plugin(app: &mut App, name: &str) {
    let command = app.commands.find(name).expect("plugin command");
    let target = match app.commands.spec(command).expect("command spec").handler {
        CommandHandler::Plugin(target) => target,
        other => panic!("unexpected handler: {other:?}"),
    };
    app.message = None;
    app.run_plugin_command(target, None).unwrap();
}

fn wait_for(app: &mut App, label: &str, predicate: impl Fn(&App) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        app.poll_plugins();
        if predicate(app) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for {label}");
}

fn wait_idle(app: &mut App) {
    wait_for(app, "plugin to finish", |app| {
        !app.plugins.in_flight.contains(PLUGIN)
    });
}

#[test]
fn wasm_plugin_controls_an_overlay_and_picker() {
    let root = temp_dir("wasm-overlay");
    write_plugin(&root);
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");

    start_plugin(&mut app, &format!("{PLUGIN}.overlay-open"));
    wait_for(&mut app, "overlay to open", |app| {
        app.plugin_overlays.view().is_some()
    });
    wait_idle(&mut app);
    let view = app.plugin_overlays.view().expect("overlay view");
    assert_eq!(view.title, "Search");
    assert_eq!(view.status, "ready");
    assert_eq!(view.lines.len(), 2);
    assert_eq!(view.lines[0].spans[0].text, "first");
    assert!(view.focus);

    start_plugin(&mut app, &format!("{PLUGIN}.overlay-update"));
    wait_for(&mut app, "overlay to update", |app| {
        app.plugin_overlays
            .view()
            .is_some_and(|view| view.status == "updated")
    });
    wait_idle(&mut app);
    let view = app.plugin_overlays.view().expect("overlay view");
    assert_eq!(view.selected, Some(1));
    assert_eq!(view.lines[0].spans[0].text, "third");

    start_plugin(&mut app, &format!("{PLUGIN}.overlay-wait"));
    wait_for(&mut app, "overlay wait", |app| {
        app.plugins.in_flight.contains(PLUGIN)
    });
    app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE))
        .unwrap();
    wait_idle(&mut app);
    assert_eq!(app.message.as_deref(), Some("key:j"));

    start_plugin(&mut app, &format!("{PLUGIN}.overlay-close"));
    wait_for(&mut app, "overlay to close", |app| {
        app.plugin_overlays.view().is_none()
    });
    wait_idle(&mut app);

    std::thread::sleep(MESSAGE_INTERVAL);
    start_plugin(&mut app, &format!("{PLUGIN}.picker"));
    wait_for(&mut app, "picker to open", |app| app.plugin_ui.has_prompt());
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    wait_idle(&mut app);
    assert_eq!(app.message.as_deref(), Some("picked:b"));
    fs::remove_dir_all(root).unwrap();
}
