use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;
use kawi_plugin::{OverlayCloseReason, OVERLAY_NOT_FOUND, UI_BUSY};
use serde_json::json;

use crate::app::App;
use crate::keymap::Flavor;

const PLUGIN: &str = "overlay";

fn app() -> App {
    App::new(Buffer::from_string("alpha"), Flavor::Vim)
}

fn open(app: &mut App, focus: bool) -> u64 {
    app.handle_plugin_request(
        PLUGIN,
        "ui.overlay_open",
        json!({
            "title": "Results",
            "status": "running",
            "anchor": "center",
            "height": 6,
            "focus": focus,
            "lines": [
                {"spans": [{"text": "one"}, {"text": " two", "style": "success"}]},
                {"spans": [{"text": "three"}]}
            ],
        }),
    )
    .expect("open overlay")
    .get("overlay")
    .and_then(serde_json::Value::as_u64)
    .expect("overlay id")
}

#[test]
fn overlay_opens_updates_and_closes() {
    let mut app = app();
    let id = open(&mut app, true);
    let view = app.plugin_overlays.view().expect("overlay view");
    assert_eq!(view.title, "Results");
    assert_eq!(view.status, "running");
    assert_eq!(view.lines.len(), 2);
    assert!(view.focus);
    assert_eq!(app.plugin_overlay_rect().height, 6);

    app.handle_plugin_request(
        PLUGIN,
        "ui.overlay_update",
        json!({
            "overlay": id,
            "title": "Done",
            "status": "",
            "selected": 1,
            "scroll": 0,
            "lines": [{"spans": [{"text": "complete", "style": "accent"}]}],
        }),
    )
    .expect("update overlay");
    let view = app.plugin_overlays.view().expect("overlay view");
    assert_eq!(view.title, "Done");
    assert_eq!(view.status, "");
    assert_eq!(view.selected, Some(1));
    assert_eq!(view.lines[0].spans[0].text, "complete");

    app.handle_plugin_request(PLUGIN, "ui.overlay_close", json!({"overlay": id}))
        .expect("close overlay");
    assert!(app.plugin_overlays.view().is_none());
}

#[test]
fn overlay_close_queues_the_plugin_close_event() {
    let mut app = app();
    let id = open(&mut app, true);
    app.handle_plugin_request(PLUGIN, "ui.overlay_close", json!({"overlay": id}))
        .expect("close overlay");

    let result = app
        .handle_plugin_prompt_request(PLUGIN, &json!(6), "ui.overlay_wait", json!({"overlay": id}))
        .expect("wait result")
        .expect("closed event");
    assert_eq!(result["kind"], "closed");
    assert_eq!(result["value"], "plugin");
}

#[test]
fn overlay_is_exclusive_and_owned_by_its_plugin() {
    let mut app = app();
    let id = open(&mut app, false);
    let error = app
        .handle_plugin_request(
            "other",
            "ui.overlay_open",
            json!({"title": "Other", "lines": []}),
        )
        .unwrap_err();
    assert_eq!(error.code, UI_BUSY);
    let error = app
        .handle_plugin_request(
            "other",
            "ui.overlay_update",
            json!({"overlay": id, "title": "Other"}),
        )
        .unwrap_err();
    assert_eq!(error.code, OVERLAY_NOT_FOUND);
}

#[test]
fn overlay_rejects_open_while_a_prompt_is_active() {
    let mut app = app();
    let pending = app.handle_plugin_prompt_request(
        PLUGIN,
        &json!(7),
        "ui.input",
        json!({"title": "Name", "value": ""}),
    );
    assert!(pending.is_none());

    let error = app
        .handle_plugin_request(
            PLUGIN,
            "ui.overlay_open",
            json!({"title": "Menu", "lines": []}),
        )
        .unwrap_err();
    assert_eq!(error.code, UI_BUSY);
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        .unwrap();
}

#[test]
fn picker_and_wait_reject_a_competing_interactive_surface() {
    let mut app = app();
    let id = open(&mut app, true);
    let error = app
        .handle_plugin_prompt_request(
            PLUGIN,
            &json!(8),
            "ui.picker",
            json!({"title": "Pick", "items": [{"id": "a", "label": "Alpha"}]}),
        )
        .expect("picker error")
        .unwrap_err();
    assert_eq!(error.code, UI_BUSY);
    app.handle_plugin_request(PLUGIN, "ui.overlay_close", json!({"overlay": id}))
        .expect("close overlay");

    let pending = app.handle_plugin_prompt_request(
        PLUGIN,
        &json!(9),
        "ui.input",
        json!({"title": "Name", "value": ""}),
    );
    assert!(pending.is_none());
    let error = app
        .handle_plugin_prompt_request(
            PLUGIN,
            &json!(10),
            "ui.overlay_wait",
            json!({"overlay": id}),
        )
        .expect("wait error")
        .unwrap_err();
    assert_eq!(error.code, UI_BUSY);
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        .unwrap();
}

#[test]
fn escape_close_reaches_a_waiter() {
    let mut app = app();
    let id = open(&mut app, true);
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        .unwrap();
    assert!(app.plugin_overlays.view().is_none());

    let result = app
        .handle_plugin_prompt_request(PLUGIN, &json!(1), "ui.overlay_wait", json!({"overlay": id}))
        .expect("wait result")
        .expect("closed event");
    assert_eq!(result["kind"], "closed");
    assert_eq!(result["value"], "escape");
}

#[test]
fn closed_event_survives_another_plugins_overlay() {
    let mut app = app();
    let id = open(&mut app, true);
    assert!(app.close_plugin_overlay(OverlayCloseReason::Escape));
    app.handle_plugin_request(
        "other",
        "ui.overlay_open",
        json!({"title": "Other", "lines": []}),
    )
    .expect("open second overlay");

    let result = app
        .handle_plugin_prompt_request(PLUGIN, &json!(5), "ui.overlay_wait", json!({"overlay": id}))
        .expect("wait result")
        .expect("closed event");
    assert_eq!(result["kind"], "closed");
    assert_eq!(result["value"], "escape");
}

#[test]
fn wait_blocks_until_a_key_arrives() {
    let mut app = app();
    let id = open(&mut app, true);
    let pending = app.handle_plugin_prompt_request(
        PLUGIN,
        &json!(2),
        "ui.overlay_wait",
        json!({"overlay": id}),
    );
    assert!(pending.is_none());

    app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE))
        .unwrap();
    assert!(app.plugin_overlays.view().is_some());

    app.close_plugin_overlay(OverlayCloseReason::Command);
    assert!(app.plugin_overlays.view().is_none());
}

#[test]
fn picker_opens_and_selects_an_item() {
    let mut app = app();
    let pending = app.handle_plugin_prompt_request(
        PLUGIN,
        &json!(3),
        "ui.picker",
        json!({
            "title": "Pick",
            "items": [
                {"id": "a", "label": "Alpha"},
                {"id": "b", "label": "Beta", "detail": "second"}
            ],
            "selected": 1,
        }),
    );
    assert!(pending.is_none());
    assert!(app.plugin_ui.picker().is_some());

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    assert!(!app.plugin_ui.has_prompt());
}

#[test]
fn picker_cancel_clears_the_prompt() {
    let mut app = app();
    let pending = app.handle_plugin_prompt_request(
        PLUGIN,
        &json!(11),
        "ui.picker",
        json!({
            "title": "Pick",
            "items": [{"id": "a", "label": "Alpha"}],
        }),
    );
    assert!(pending.is_none());
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        .unwrap();
    assert!(!app.plugin_ui.has_prompt());
}

#[test]
fn picker_rejects_empty_items() {
    let mut app = app();
    let result = app
        .handle_plugin_prompt_request(
            PLUGIN,
            &json!(4),
            "ui.picker",
            json!({"title": "Pick", "items": []}),
        )
        .expect("picker error")
        .unwrap_err();
    assert_eq!(result.code, kawi_plugin::INVALID_PARAMS);
}
