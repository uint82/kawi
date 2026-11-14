use std::collections::BTreeMap;
use std::fs;

use kawi_plugin::{
    BufferInfo as PluginBufferInfo, BufferState, Capability, EffectiveCapabilities,
    HostEnvironment, Scope, BUFFER_NOT_ACTIVE, BUFFER_NOT_FOUND, CAPABILITY_DENIED,
    MODE_NOT_SUPPORTED, OPTION_NOT_FOUND,
};
use serde_json::{json, Value};

use super::common::{app, temp_dir};
use crate::app::App;

mod events;
mod windows;

const PLUGIN: &str = "test";

fn grant(app: &mut App, capability: Capability, scope: Scope) {
    let mut granted = BTreeMap::new();
    granted.insert(capability, scope);
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
