use std::time::{Duration, Instant};

use kawi_plugin::{
    Capability, Scope, TimerInfo, Variable, BUFFER_NOT_FOUND, INVALID_PARAMS, OPTION_NOT_FOUND,
    WINDOW_NOT_FOUND,
};
use serde_json::{json, Value};

use super::{app, grant, request};

#[test]
fn schedules_lists_and_cancels_timers() {
    let mut app = app();
    grant(&mut app, Capability::EditorSchedule, Scope::Unscoped);

    let value = request(
        &mut app,
        "timer.after",
        json!({ "delay_ms": 60_000, "payload": "later" }),
    )
    .expect("after timer");
    let after = value.as_u64().expect("timer id");

    let value = request(
        &mut app,
        "timer.every",
        json!({ "interval_ms": 10, "payload": "tick" }),
    )
    .expect("every timer");
    let every = value.as_u64().expect("timer id");

    let value = request(&mut app, "timer.list", Value::Null).expect("list timers");
    let timers: Vec<TimerInfo> = serde_json::from_value(value).expect("timers");
    assert_eq!(timers.len(), 2);
    assert_eq!(timers[0].id, after);
    assert_eq!(timers[0].payload.as_deref(), Some("later"));
    assert_eq!(timers[1].id, every);
    assert_eq!(timers[1].interval_ms, Some(10));

    let value = request(&mut app, "timer.cancel", json!({ "timer": after })).expect("cancel");
    assert_eq!(value, Value::Bool(true));
    let value = request(&mut app, "timer.cancel", json!({ "timer": after })).expect("cancel");
    assert_eq!(value, Value::Bool(false));
    let value = request(&mut app, "timer.list", Value::Null).expect("list timers");
    let timers: Vec<TimerInfo> = serde_json::from_value(value).expect("timers");
    assert_eq!(timers.len(), 1);

    for params in [
        json!({ "delay_ms": 86_400_001 }),
        json!({ "delay_ms": 1, "payload": "x".repeat(4_097) }),
    ] {
        let error = request(&mut app, "timer.after", params).expect_err("invalid after timer");
        assert_eq!(error.code, INVALID_PARAMS);
    }
    for params in [
        json!({ "interval_ms": 9 }),
        json!({ "interval_ms": 86_400_001 }),
    ] {
        let error = request(&mut app, "timer.every", params).expect_err("invalid every timer");
        assert_eq!(error.code, INVALID_PARAMS);
    }
}

#[test]
fn dispatches_due_timers_and_advances_repeating_timers() {
    let mut app = app();
    grant(&mut app, Capability::EditorSchedule, Scope::Unscoped);

    request(&mut app, "timer.after", json!({ "delay_ms": 60_000 })).expect("after timer");
    let now = Instant::now();
    let deadline = app.plugins.timer_deadline(now).expect("deadline");
    assert!(deadline <= Duration::from_millis(60_000));
    app.plugins.dispatch_due_timers(now);
    let value = request(&mut app, "timer.list", Value::Null).expect("list timers");
    let timers: Vec<TimerInfo> = serde_json::from_value(value).expect("timers");
    assert_eq!(timers.len(), 1);

    request(&mut app, "timer.after", json!({ "delay_ms": 0 })).expect("after timer");
    app.plugins.dispatch_due_timers(Instant::now());
    let value = request(&mut app, "timer.list", Value::Null).expect("list timers");
    let timers: Vec<TimerInfo> = serde_json::from_value(value).expect("timers");
    assert_eq!(timers.len(), 1);
    assert_eq!(timers[0].interval_ms, None);

    request(&mut app, "timer.every", json!({ "interval_ms": 10 })).expect("every timer");
    let now = Instant::now();
    let due = now + Duration::from_millis(10);
    app.plugins.dispatch_due_timers(due);
    let value = request(&mut app, "timer.list", Value::Null).expect("list timers");
    let timers: Vec<TimerInfo> = serde_json::from_value(value).expect("timers");
    assert_eq!(timers.len(), 2);
    assert!(app.plugins.timer_deadline(due).is_some());
}

#[test]
fn stores_variables_in_each_target_scope() {
    let mut app = app();
    grant(&mut app, Capability::EditorVariables, Scope::Unscoped);

    request(
        &mut app,
        "variable.set",
        json!({ "target": { "scope": "global" }, "key": "alpha", "value": true }),
    )
    .expect("set global");
    request(
        &mut app,
        "variable.set",
        json!({ "target": { "scope": "plugin" }, "key": "alpha", "value": "plugin" }),
    )
    .expect("set plugin");
    request(
        &mut app,
        "variable.set",
        json!({ "target": { "scope": "buffer", "buffer": 1 }, "key": "alpha", "value": 3 }),
    )
    .expect("set buffer");
    request(
        &mut app,
        "variable.set",
        json!({ "target": { "scope": "window", "window": 1 }, "key": "alpha", "value": 4.5 }),
    )
    .expect("set window");

    let value = request(
        &mut app,
        "variable.get",
        json!({ "target": { "scope": "global" }, "key": "alpha" }),
    )
    .expect("get global");
    assert_eq!(value, Value::Bool(true));

    let value = request(
        &mut app,
        "variable.list",
        json!({ "target": { "scope": "buffer", "buffer": 1 } }),
    )
    .expect("list buffer");
    let variables: Vec<Variable> = serde_json::from_value(value).expect("variables");
    assert_eq!(variables.len(), 1);
    assert_eq!(variables[0].key, "alpha");
    assert_eq!(variables[0].value, json!(3));

    let value = request(
        &mut app,
        "variable.delete",
        json!({ "target": { "scope": "window", "window": 1 }, "key": "alpha" }),
    )
    .expect("delete window");
    assert_eq!(value, Value::Bool(true));
    let value = request(
        &mut app,
        "variable.get",
        json!({ "target": { "scope": "window", "window": 1 }, "key": "alpha" }),
    )
    .expect("get deleted");
    assert_eq!(value, Value::Null);

    for (params, code) in [
        (
            json!({ "target": { "scope": "buffer", "buffer": 99 }, "key": "alpha", "value": 1 }),
            BUFFER_NOT_FOUND,
        ),
        (
            json!({ "target": { "scope": "window", "window": 99 }, "key": "alpha", "value": 1 }),
            WINDOW_NOT_FOUND,
        ),
        (
            json!({ "target": { "scope": "global", "buffer": 1 }, "key": "alpha", "value": 1 }),
            INVALID_PARAMS,
        ),
        (
            json!({ "target": { "scope": "buffer" }, "key": "alpha", "value": 1 }),
            INVALID_PARAMS,
        ),
        (
            json!({ "target": { "scope": "global" }, "key": "", "value": 1 }),
            INVALID_PARAMS,
        ),
        (
            json!({ "target": { "scope": "global" }, "key": "alpha", "value": { "a": 1 } }),
            INVALID_PARAMS,
        ),
    ] {
        let error = request(&mut app, "variable.set", params).expect_err("invalid variable");
        assert_eq!(error.code, code);
    }
}

#[test]
fn writes_and_resets_options() {
    let mut app = app();
    grant(&mut app, Capability::OptionsWrite, Scope::Unscoped);

    let value = request(
        &mut app,
        "options.set",
        json!({ "name": "nu", "value": false }),
    )
    .expect("set option");
    let info: kawi_plugin::OptionInfo = serde_json::from_value(value).expect("option");
    assert_eq!(info.name, "number");
    assert_eq!(info.value, "false");

    let value = request(&mut app, "options.reset", json!({ "name": "number" })).expect("reset");
    let info: kawi_plugin::OptionInfo = serde_json::from_value(value).expect("option");
    assert_eq!(info.value, "true");

    let error = request(
        &mut app,
        "options.set",
        json!({ "name": "nope", "value": true }),
    )
    .expect_err("unknown option");
    assert_eq!(error.code, OPTION_NOT_FOUND);

    for value in [Value::Null, json!({}), json!(1.5), json!([true])] {
        let error = request(
            &mut app,
            "options.set",
            json!({ "name": "number", "value": value }),
        )
        .expect_err("invalid value");
        assert_eq!(error.code, INVALID_PARAMS);
    }
    let error = request(
        &mut app,
        "options.set",
        json!({ "name": "number", "value": "text" }),
    )
    .expect_err("wrong type");
    assert_eq!(error.code, INVALID_PARAMS);
}

#[test]
fn clears_buffer_and_window_variables_when_targets_close() {
    let mut app = app();
    grant(&mut app, Capability::EditorVariables, Scope::Unscoped);

    request(
        &mut app,
        "variable.set",
        json!({ "target": { "scope": "buffer", "buffer": 1 }, "key": "alpha", "value": 1 }),
    )
    .expect("set buffer variable");
    let view = *app.windows.active_view();
    let window = app
        .windows
        .split_active(crate::app::window::SplitDirection::Vertical, view);
    request(
        &mut app,
        "variable.set",
        json!({ "target": { "scope": "window", "window": window }, "key": "alpha", "value": 2 }),
    )
    .expect("set window variable");

    app.editor.open_empty_buffer();
    app.editor.close_buffer(1, true).expect("close buffer");
    app.close_active_window(true).expect("close window");
    app.poll_plugins();

    assert!(!app.plugins.buffer_variables.contains_key(&1));
    assert!(!app.plugins.window_variables.contains_key(&window));
}
