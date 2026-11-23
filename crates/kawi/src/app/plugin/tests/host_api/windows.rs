use kawi_plugin::{Capability, Scope, BUFFER_MODIFIED, LAST_WINDOW, WINDOW_NOT_FOUND};
use serde_json::{json, Value};

use super::{app, grant, request};

#[test]
fn opens_focuses_and_closes_windows() {
    let mut app = app();
    grant(&mut app, Capability::EditorWindows, Scope::Unscoped);

    let value = request(&mut app, "window.list", Value::Null).expect("list windows");
    let infos: Vec<kawi_plugin::WindowInfo> = serde_json::from_value(value).expect("windows");
    assert_eq!(infos.len(), 1);
    assert!(infos[0].active);

    let value = request(
        &mut app,
        "window.open",
        json!({ "buffer": 1, "direction": "vertical" }),
    )
    .expect("open split");
    let opened: kawi_plugin::WindowInfo = serde_json::from_value(value).expect("window");
    assert!(opened.active);
    assert_eq!(app.windows.len(), 2);

    let _ = request(&mut app, "window.focus", json!({ "window": 1 })).expect("focus");
    assert_eq!(app.windows.active(), 1);

    let value = request(
        &mut app,
        "window.close",
        json!({ "window": opened.window, "force": false }),
    )
    .expect("close split");
    let remaining: kawi_plugin::WindowInfo = serde_json::from_value(value).expect("window");
    assert_eq!(remaining.window, 1);
    assert_eq!(app.windows.len(), 1);

    let error = request(
        &mut app,
        "window.close",
        json!({ "window": 1, "force": false }),
    )
    .expect_err("last window");
    assert_eq!(error.code, LAST_WINDOW);

    let error =
        request(&mut app, "window.get", json!({ "window": 99 })).expect_err("unknown window");
    assert_eq!(error.code, WINDOW_NOT_FOUND);
}

#[test]
fn closing_a_window_with_unsaved_changes_requires_force() {
    let mut app = app();
    grant(&mut app, Capability::EditorWindows, Scope::Unscoped);

    let value = request(
        &mut app,
        "window.open",
        json!({ "buffer": 1, "direction": "vertical" }),
    )
    .expect("open split");
    let opened: kawi_plugin::WindowInfo = serde_json::from_value(value).expect("window");
    app.editor.apply(kawi_editor::Action::EnterInsert);
    app.editor.apply(kawi_editor::Action::InsertChar('x'));
    app.editor.apply(kawi_editor::Action::ExitInsert);

    let error = request(
        &mut app,
        "window.close",
        json!({ "window": opened.window, "force": false }),
    )
    .expect_err("modified buffer");
    assert_eq!(error.code, BUFFER_MODIFIED);

    request(
        &mut app,
        "window.close",
        json!({ "window": opened.window, "force": true }),
    )
    .expect("forced close");
    assert_eq!(app.windows.len(), 1);
    assert_eq!(app.editor.buffer_is_dirty(1), Some(true));
}
