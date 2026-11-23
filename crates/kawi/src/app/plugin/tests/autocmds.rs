use std::collections::BTreeMap;
use std::path::Path;

use kawi_plugin::{Capability, EffectiveCapabilities, Scope, CAPABILITY_DENIED, INVALID_PARAMS};
use serde_json::json;

use super::common::{app, load_plugins, manifest, temp_dir};
use crate::app::App;

#[test]
fn registering_an_event_without_its_capability_is_denied() {
    let root = temp_dir("autocmd-capability");
    let mut app = loaded_app(&root);
    grant_autocmds(&mut app, "echo");

    let error = app
        .handle_plugin_request("echo", "autocmd.register", json!({ "event": "file_saved" }))
        .expect_err("file_saved requires editor.read");

    assert_eq!(error.code, CAPABILITY_DENIED);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn registering_an_unknown_event_is_invalid_params() {
    let root = temp_dir("autocmd-event");
    let mut app = loaded_app(&root);
    grant_autocmds(&mut app, "echo");

    let error = app
        .handle_plugin_request(
            "echo",
            "autocmd.register",
            json!({ "event": "not_an_event" }),
        )
        .expect_err("unknown event");

    assert_eq!(error.code, INVALID_PARAMS);
    std::fs::remove_dir_all(root).unwrap();
}

fn loaded_app(root: &Path) -> App {
    manifest(root);
    let mut app = app();
    load_plugins(&mut app, root);
    app
}

fn grant_autocmds(app: &mut App, plugin: &str) {
    let granted = BTreeMap::from([(Capability::EditorAutocmds, Scope::Unscoped)]);
    app.plugins.capabilities.replace(BTreeMap::from([(
        plugin.to_string(),
        EffectiveCapabilities { granted },
    )]));
}
