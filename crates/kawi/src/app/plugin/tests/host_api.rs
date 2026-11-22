use std::collections::BTreeMap;
use std::fs;

use kawi_plugin::{
    BufferInfo as PluginBufferInfo, BufferState, Capability, EffectiveCapabilities,
    HostEnvironment, PluginHostEvent, PluginStatus, Scope, BUFFER_MODIFIED, BUFFER_NOT_ACTIVE,
    BUFFER_NOT_FOUND, BUFFER_NOT_OWNED, CAPABILITY_DENIED, MODE_NOT_SUPPORTED, OPTION_NOT_FOUND,
};
use serde_json::{json, Value};

use super::common::{app, temp_dir};
use crate::app::App;

mod automation;
mod events;
mod windows;

const PLUGIN: &str = "test";

fn grant(app: &mut App, capability: Capability, scope: Scope) {
    grant_all(app, [(capability, scope)]);
}

fn grant_all(app: &mut App, capabilities: impl IntoIterator<Item = (Capability, Scope)>) {
    let granted = capabilities.into_iter().collect::<BTreeMap<_, _>>();
    app.plugins.capabilities.replace(BTreeMap::from([(
        PLUGIN.to_string(),
        EffectiveCapabilities { granted },
    )]));
}

fn request(app: &mut App, method: &str, params: Value) -> Result<Value, kawi_plugin::RpcError> {
    app.handle_plugin_request(PLUGIN, method, params)
}

#[test]
fn lists_buffers_and_reads_their_state() {
    let mut app = app();
    let value = request(&mut app, "buffer.list", Value::Null).expect("list");
    let infos: Vec<PluginBufferInfo> = serde_json::from_value(value).expect("buffer infos");
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].buffer, 1);
    assert!(infos[0].path.is_none());

    let value = request(
        &mut app,
        "buffer.get",
        json!({ "buffer": 1, "include_text": true }),
    )
    .expect("get");
    let state: BufferState = serde_json::from_value(value).expect("buffer state");
    assert_eq!(state.text.as_deref(), Some("alpha"));
    assert!(!state.modified);

    let error =
        request(&mut app, "buffer.get", json!({ "buffer": 99 })).expect_err("unknown buffer");
    assert_eq!(error.code, BUFFER_NOT_FOUND);
}

#[test]
fn controls_the_active_buffer_cursor_selection_and_mode() {
    let mut app = app();
    grant(&mut app, Capability::EditorControl, Scope::Unscoped);

    let value = request(
        &mut app,
        "editor.set_cursor",
        json!({ "buffer": 1, "cursor": { "line": 0, "byte_column": 2 } }),
    )
    .expect("set cursor");
    let state: kawi_plugin::GetStateResult = serde_json::from_value(value).expect("state");
    assert_eq!(state.cursor.byte_column, 2);

    let value = request(
        &mut app,
        "editor.set_selection",
        json!({
            "buffer": 1,
            "selection": {
                "start": { "line": 0, "byte_column": 0 },
                "end": { "line": 0, "byte_column": 3 }
            }
        }),
    )
    .expect("set selection");
    let state: kawi_plugin::GetStateResult = serde_json::from_value(value).expect("state");
    assert_eq!(state.mode, "visual");
    assert!(state.selection.is_some());

    let value = request(
        &mut app,
        "editor.set_selection",
        json!({ "buffer": 1, "selection": null }),
    )
    .expect("clear selection");
    let state: kawi_plugin::GetStateResult = serde_json::from_value(value).expect("state");
    assert_eq!(state.mode, "normal");
    assert!(state.selection.is_none());

    let error = request(
        &mut app,
        "editor.set_mode",
        json!({ "buffer": 1, "mode": "command" }),
    )
    .expect_err("input mode is not supported");
    assert_eq!(error.code, MODE_NOT_SUPPORTED);

    app.editor.open_empty_buffer();
    let error = request(
        &mut app,
        "editor.set_cursor",
        json!({ "buffer": 1, "cursor": { "line": 0, "byte_column": 0 } }),
    )
    .expect_err("inactive buffer");
    assert_eq!(error.code, BUFFER_NOT_ACTIVE);
}

#[test]
fn lists_and_reads_options_by_name_or_alias() {
    let mut app = app();
    grant(&mut app, Capability::OptionsRead, Scope::Unscoped);

    let value =
        request(&mut app, "options.list", json!({ "prefix": "number" })).expect("list options");
    let infos: Vec<kawi_plugin::OptionInfo> = serde_json::from_value(value).expect("options");
    let names = infos
        .iter()
        .map(|info| info.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["number", "numberwidth"]);

    let value = request(&mut app, "options.get", json!({ "name": "sw" })).expect("alias");
    let info: kawi_plugin::OptionInfo = serde_json::from_value(value).expect("option");
    assert_eq!(info.name, "shiftwidth");
    assert_eq!(info.kind, "integer");
    assert_eq!(info.scope, "buffer");
    assert_eq!(info.section, "editor");

    let error =
        request(&mut app, "options.get", json!({ "name": "nope" })).expect_err("unknown option");
    assert_eq!(error.code, OPTION_NOT_FOUND);
}

#[test]
fn authorizes_buffer_writes_by_path_scope() {
    let root = temp_dir("host_api_paths").canonicalize().unwrap();
    let outside = root.parent().unwrap().join(format!(
        "{}_outside.txt",
        root.file_name().unwrap().to_string_lossy()
    ));
    fs::write(root.join("allowed.txt"), "hello").unwrap();
    fs::write(&outside, "outside").unwrap();
    let plugin_dir = root.join("echo");
    fs::create_dir_all(&plugin_dir).unwrap();
    fs::write(
        plugin_dir.join("plugin.toml"),
        "name = \"test\"\nversion = \"0.1.0\"\nprotocol = 1\ncommand = [\"/bin/true\"]\n",
    )
    .unwrap();

    let mut app = app();
    app.plugins.catalog = kawi_plugin::load_catalog(&root);
    app.plugins.environment = Some(HostEnvironment::from_paths(Some(root.clone()), None));
    grant(
        &mut app,
        Capability::EditorBuffersWrite,
        Scope::Paths(vec!["$WORKSPACE".to_string()]),
    );

    let value = request(
        &mut app,
        "buffer.open",
        json!({ "path": "$WORKSPACE/allowed.txt" }),
    )
    .expect("allowed path");
    let state: BufferState = serde_json::from_value(value).expect("buffer state");
    assert_eq!(
        state.path.as_deref(),
        Some(root.join("allowed.txt").to_str().unwrap())
    );

    let revision = state.revision;
    let value = request(
        &mut app,
        "buffer.apply_edit",
        json!({
            "buffer": state.buffer,
            "revision": revision,
            "edits": [{
                "start": { "line": 0, "byte_column": 5 },
                "end": { "line": 0, "byte_column": 5 },
                "text": "!"
            }]
        }),
    )
    .expect("apply edit");
    let result: kawi_plugin::ApplyEditResult = serde_json::from_value(value).expect("result");
    assert!(result.revision > revision);
    assert_eq!(app.editor.document_snapshot().to_string(), "hello!");

    let relative = format!(
        "$WORKSPACE/../{}",
        outside.file_name().unwrap().to_string_lossy()
    );
    let error =
        request(&mut app, "buffer.open", json!({ "path": relative })).expect_err("outside scope");
    assert_eq!(error.code, CAPABILITY_DENIED);

    fs::remove_dir_all(root).unwrap();
    let _ = fs::remove_file(outside);
}

#[test]
fn creates_and_closes_owned_scratch_buffers() {
    let mut app = app();
    grant(&mut app, Capability::EditorBuffersWrite, Scope::Unscoped);

    let value = request(
        &mut app,
        "buffer.create",
        json!({ "text": "scratch text\n" }),
    )
    .expect("create");
    let state: BufferState = serde_json::from_value(value).expect("buffer state");
    assert_ne!(state.buffer, 1);
    assert_eq!(state.path, None);
    assert_eq!(state.text, None);
    assert!(!state.modified);
    assert_eq!(app.editor.buffer_number(), 1);

    let value = request(
        &mut app,
        "buffer.get",
        json!({ "buffer": state.buffer, "include_text": true }),
    )
    .expect("get scratch");
    let text: BufferState = serde_json::from_value(value).expect("buffer state");
    assert_eq!(text.text.as_deref(), Some("scratch text\n"));

    request(&mut app, "buffer.close", json!({ "buffer": state.buffer })).expect("close");
    let error = request(&mut app, "buffer.get", json!({ "buffer": state.buffer }))
        .expect_err("closed buffer");
    assert_eq!(error.code, BUFFER_NOT_FOUND);
}

#[test]
fn closing_an_owned_buffer_replaces_it_in_every_window() {
    let mut app = app();
    grant_all(
        &mut app,
        [
            (Capability::EditorBuffersWrite, Scope::Unscoped),
            (Capability::EditorWindows, Scope::Unscoped),
        ],
    );
    let value = request(&mut app, "buffer.create", json!({ "text": "note\n" })).expect("create");
    let state: BufferState = serde_json::from_value(value).expect("buffer state");
    request(
        &mut app,
        "window.open",
        json!({ "buffer": state.buffer, "direction": "vertical" }),
    )
    .expect("open window");
    assert_eq!(app.windows.count_buffer(state.buffer as usize), 1);

    request(&mut app, "buffer.close", json!({ "buffer": state.buffer })).expect("close");
    assert_eq!(app.windows.count_buffer(state.buffer as usize), 0);
    assert_eq!(app.windows.count_buffer(1), 2);
}

#[test]
fn buffer_lifecycle_requires_the_buffer_write_capability() {
    let mut app = app();
    for method in ["buffer.create", "buffer.close"] {
        assert_eq!(
            app.plugins.denied_capability(PLUGIN, method),
            Some(Capability::EditorBuffersWrite)
        );
    }
    grant(&mut app, Capability::EditorBuffersWrite, Scope::Unscoped);
    for method in ["buffer.create", "buffer.close"] {
        assert_eq!(app.plugins.denied_capability(PLUGIN, method), None);
    }
}

#[test]
fn rejects_closing_buffers_the_plugin_does_not_own() {
    let mut app = app();
    grant(&mut app, Capability::EditorBuffersWrite, Scope::Unscoped);

    let error = request(&mut app, "buffer.close", json!({ "buffer": 1 })).expect_err("user buffer");
    assert_eq!(error.code, BUFFER_NOT_OWNED);
    assert!(app.editor.buffer_is_dirty(1).is_some());

    let error = request(&mut app, "buffer.close", json!({ "buffer": 99 })).expect_err("unknown");
    assert_eq!(error.code, BUFFER_NOT_OWNED);
}

#[test]
fn closing_a_vanished_buffer_releases_ownership() {
    let mut app = app();
    grant(&mut app, Capability::EditorBuffersWrite, Scope::Unscoped);
    let value = request(&mut app, "buffer.create", json!({ "text": "gone\n" })).expect("create");
    let state: BufferState = serde_json::from_value(value).expect("buffer state");
    app.editor
        .close_buffer(state.buffer as usize, true)
        .unwrap();

    let error = request(&mut app, "buffer.close", json!({ "buffer": state.buffer }))
        .expect_err("vanished buffer");
    assert_eq!(error.code, BUFFER_NOT_FOUND);
    assert!(!app.plugins.owns_buffer(PLUGIN, state.buffer));
}

#[test]
fn closing_a_modified_owned_buffer_requires_force() {
    let mut app = app();
    grant(&mut app, Capability::EditorBuffersWrite, Scope::Unscoped);
    let value = request(&mut app, "buffer.create", json!({ "text": "note\n" })).expect("create");
    let state: BufferState = serde_json::from_value(value).expect("buffer state");
    app.editor.buffer_select(state.buffer as usize);
    app.editor.apply(kawi_editor::Action::EnterInsert);
    app.editor.apply(kawi_editor::Action::InsertChar('x'));
    app.editor.apply(kawi_editor::Action::ExitInsert);

    let error = request(&mut app, "buffer.close", json!({ "buffer": state.buffer }))
        .expect_err("modified buffer");
    assert_eq!(error.code, BUFFER_MODIFIED);

    request(
        &mut app,
        "buffer.close",
        json!({ "buffer": state.buffer, "force": true }),
    )
    .expect("forced close");
    assert!(app.editor.buffer_is_dirty(state.buffer as usize).is_none());
}

#[test]
fn stopping_a_plugin_closes_the_buffers_it_created() {
    let mut app = app();
    grant(&mut app, Capability::EditorBuffersWrite, Scope::Unscoped);
    let value = request(&mut app, "buffer.create", json!({ "text": "temp\n" })).expect("create");
    let state: BufferState = serde_json::from_value(value).expect("buffer state");

    app.handle_plugin_event(PluginHostEvent::StatusChanged {
        plugin: PLUGIN.to_string(),
        status: PluginStatus::Stopped,
    });
    assert!(app.editor.buffer_is_dirty(state.buffer as usize).is_none());
}
