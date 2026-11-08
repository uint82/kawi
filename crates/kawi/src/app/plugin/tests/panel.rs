use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::json;

use super::common::app;
use crate::app::App;

const PLUGIN: &str = "panel";

fn open_panel(app: &mut App, focus: bool) -> u64 {
    app.handle_plugin_request(
        PLUGIN,
        "ui.panel_open",
        json!({
            "title": "Index",
            "status": "running",
            "height": 8,
            "focus": focus,
            "lines": [
                {"text": "one"},
                {"text": "two", "style": "success"}
            ],
        }),
    )
    .expect("open panel")
    .get("panel")
    .and_then(serde_json::Value::as_u64)
    .expect("panel id")
}

#[test]
fn q_closes_the_focused_panel() {
    let mut app = app();
    open_panel(&mut app, true);
    app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))
        .unwrap();
    assert!(app.plugin_panel_view().is_none());
    assert!(app.is_editor_focused());
}

#[test]
fn panel_opens_updates_and_closes() {
    let mut app = app();
    let id = open_panel(&mut app, false);
    let view = app.plugin_panel_view().expect("panel view");
    assert_eq!(view.title, "Index");
    assert_eq!(view.status, "running");
    assert_eq!(view.lines.len(), 2);
    assert!(!view.focused);

    app.handle_plugin_request(
        PLUGIN,
        "ui.panel_update",
        json!({
            "panel": id,
            "status": "done",
            "height": 6,
        }),
    )
    .expect("update panel status");
    let view = app.plugin_panel_view().expect("panel view");
    assert_eq!(view.status, "done");
    assert_eq!(view.lines[0].text, "one");
    assert_eq!(app.plugin_panel_rect().unwrap().height, 6);

    app.handle_plugin_request(
        PLUGIN,
        "ui.panel_update",
        json!({
            "panel": id,
            "title": "Index",
            "lines": [{"text": "complete", "style": "success"}],
        }),
    )
    .expect("update panel content");
    let view = app.plugin_panel_view().expect("panel view");
    assert_eq!(view.lines[0].text, "complete");

    app.handle_plugin_request(PLUGIN, "ui.panel_focus", json!({"panel": id}))
        .expect("focus panel");
    assert!(app.plugin_panel_view().expect("panel view").focused);

    app.handle_plugin_request(PLUGIN, "ui.panel_close", json!({"panel": id}))
        .expect("close panel");
    assert!(app.plugin_panel_view().is_none());
}

#[test]
fn panel_is_exclusive_and_owned_by_its_plugin() {
    let mut app = app();
    let id = open_panel(&mut app, false);
    let error = app
        .handle_plugin_request(
            "other",
            "ui.panel_open",
            json!({"title": "Other", "lines": []}),
        )
        .unwrap_err();
    assert_eq!(error.code, kawi_plugin::UI_BUSY);
    let error = app
        .handle_plugin_request(
            "other",
            "ui.panel_update",
            json!({"panel": id, "lines": []}),
        )
        .unwrap_err();
    assert_eq!(error.code, kawi_plugin::PANEL_NOT_FOUND);
}

#[test]
fn panel_validates_content_limits() {
    let mut app = app();
    let lines = vec![json!({"text": "line"}); kawi_plugin::MAX_PLUGIN_PANEL_LINES + 1];
    let error = app
        .handle_plugin_request(
            PLUGIN,
            "ui.panel_open",
            json!({"title": "Too many", "lines": lines}),
        )
        .unwrap_err();
    assert_eq!(error.code, kawi_plugin::INVALID_PARAMS);
}

#[test]
fn panel_scrolls_with_host_keys_and_closes() {
    let mut app = app();
    let lines = (0..100)
        .map(|index| json!({"text": format!("line {index}")}))
        .collect::<Vec<_>>();
    let id = app
        .handle_plugin_request(
            PLUGIN,
            "ui.panel_open",
            json!({
                "title": "Index",
                "height": 6,
                "focus": true,
                "lines": lines,
            }),
        )
        .expect("open panel")
        .get("panel")
        .and_then(serde_json::Value::as_u64)
        .expect("panel id");

    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.plugin_panel.clamped_offset(5), 1);
    app.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.plugin_panel.clamped_offset(5), 95);
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        .unwrap();
    assert!(!app.plugin_panel.is_focused());
    assert!(app.plugin_panel_view().is_some());

    app.handle_plugin_request(PLUGIN, "ui.panel_focus", json!({"panel": id}))
        .expect("focus panel");
    app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE))
        .unwrap();
    assert!(app.plugin_panel_view().is_none());
}

#[test]
fn content_replacement_clamps_scroll_position() {
    let mut app = app();
    let lines = (0..100)
        .map(|index| json!({"text": format!("line {index}")}))
        .collect::<Vec<_>>();
    let id = app
        .handle_plugin_request(
            PLUGIN,
            "ui.panel_open",
            json!({"title": "Index", "height": 6, "focus": true, "lines": lines}),
        )
        .expect("open panel")
        .get("panel")
        .and_then(serde_json::Value::as_u64)
        .expect("panel id");
    app.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.plugin_panel.clamped_offset(5), 95);

    app.handle_plugin_request(
        PLUGIN,
        "ui.panel_update",
        json!({"panel": id, "lines": [{"text": "short"}]}),
    )
    .expect("short update");
    assert_eq!(app.plugin_panel.clamped_offset(5), 0);

    app.handle_plugin_request(
        PLUGIN,
        "ui.panel_update",
        json!({"panel": id, "lines": (0..100).map(|index| json!({"text": format!("line {index}")})).collect::<Vec<_>>()}),
    )
    .expect("long update");
    assert_eq!(app.plugin_panel.clamped_offset(5), 0);
}

#[test]
fn pager_takes_focus_from_the_panel() {
    let mut app = app();
    open_panel(&mut app, true);
    assert!(app.plugin_panel.is_focused());
    app.show_pager(vec!["help".to_string()]);
    assert!(!app.plugin_panel.is_focused());
}

#[test]
fn stopping_a_plugin_removes_its_panel() {
    let mut app = app();
    open_panel(&mut app, false);
    app.handle_plugin_event(kawi_plugin::PluginHostEvent::StatusChanged {
        plugin: PLUGIN.to_string(),
        status: kawi_plugin::PluginStatus::Stopped,
    });
    assert!(app.plugin_panel_view().is_none());
}

#[test]
fn panel_hides_when_the_screen_has_no_room() {
    let mut app = app();
    app.set_screen_size(40, 8);
    let result = app
        .handle_plugin_request(
            PLUGIN,
            "ui.panel_open",
            json!({"title": "Index", "height": 8, "focus": true, "lines": [{"text": "one"}]}),
        )
        .expect("open panel");
    assert_eq!(
        result.get("height").and_then(serde_json::Value::as_u64),
        Some(0)
    );
    assert!(app.plugin_panel_view().is_none());
    assert!(!app.plugin_panel.is_focused());
}
