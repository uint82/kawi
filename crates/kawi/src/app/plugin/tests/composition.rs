use std::collections::BTreeMap;
use std::fs;

use kawi_buffer::Buffer;
use kawi_editor::Mode;
use kawi_plugin::{
    Capability, EditorCommandInfo, EffectiveCapabilities, Scope, CAPABILITY_DENIED, INVALID_PARAMS,
};
use serde_json::{json, Value};

use super::common::{app, load_plugins, temp_dir};
use crate::app::App;
use crate::keymap::Flavor;

const PLUGIN: &str = "test";

fn app_with_commands(commands: &[&str]) -> App {
    let mut app = App::new(Buffer::from_string("alpha"), Flavor::Vim);
    grant_commands(&mut app, commands);
    app
}

fn grant_commands(app: &mut App, commands: &[&str]) {
    let mut granted = BTreeMap::new();
    granted.insert(
        Capability::EditorExecute,
        Scope::EditorCommands(
            commands
                .iter()
                .map(|command| (*command).to_string())
                .collect(),
        ),
    );
    app.plugins.capabilities.replace(BTreeMap::from([(
        PLUGIN.to_string(),
        EffectiveCapabilities { granted },
    )]));
}

fn execute(app: &mut App, command: &str) -> Result<Value, kawi_plugin::RpcError> {
    app.handle_plugin_request(PLUGIN, "editor.execute", json!({ "command": command }))
}

fn list_commands(app: &mut App) -> Vec<EditorCommandInfo> {
    let value = app
        .handle_plugin_request(PLUGIN, "editor.list_commands", json!({}))
        .expect("list commands");
    serde_json::from_value::<kawi_plugin::EditorListCommandsResult>(value)
        .expect("command list")
        .commands
}

#[test]
fn lists_only_allowed_composable_commands() {
    let mut app = app_with_commands(&["file.save", "edit.undo", "echo.ping", "keys.send"]);
    app.commands
        .register_plugin(
            "echo.ping".to_string(),
            "Ping".to_string(),
            "echo".to_string(),
            0,
        )
        .unwrap();
    let commands = list_commands(&mut app);
    let ids = commands
        .iter()
        .map(|command| command.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["edit.undo", "file.save"]);
    assert!(commands[1].aliases.contains(&"save".to_string()));
}

#[test]
fn executes_allowed_commands_by_canonical_name_and_alias() {
    let mut app = app_with_commands(&["mode.insert", "edit.undo"]);
    execute(&mut app, "mode.insert").expect("canonical");
    assert_eq!(app.editor.mode(), Mode::Insert);
    execute(&mut app, "undo").expect("alias");
    execute(&mut app, "mode.normal").expect_err("mode.normal is not allowed");
    assert_eq!(app.editor.mode(), Mode::Insert);
}

#[test]
fn rejects_commands_outside_the_scope() {
    let mut app = app_with_commands(&["edit.undo"]);
    let error = execute(&mut app, "edit.delete_line").expect_err("out of scope");
    assert_eq!(error.code, CAPABILITY_DENIED);
    assert_eq!(app.editor.text_lines(), vec!["alpha"]);
}

#[test]
fn rejects_unknown_and_non_composable_commands() {
    let mut app = app_with_commands(&["missing.command", "echo.ping", "keys.send"]);
    app.commands
        .register_plugin(
            "echo.ping".to_string(),
            "Ping".to_string(),
            "echo".to_string(),
            0,
        )
        .unwrap();
    for command in ["missing.command", "echo.ping", "keys.send"] {
        let error = execute(&mut app, command).expect_err(command);
        assert_eq!(error.code, INVALID_PARAMS, "{command}");
    }
}

#[test]
fn an_empty_scope_lists_nothing() {
    let mut app = App::new(Buffer::from_string("alpha"), Flavor::Vim);
    app.plugins.capabilities.replace(BTreeMap::new());
    assert!(list_commands(&mut app).is_empty());
}

#[test]
fn manifest_grants_the_editor_command_scope() {
    let root = temp_dir("editor-execute-manifest");
    let dir = root.join("compose");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        "name = \"compose\"\nversion = \"0.1.0\"\nprotocol = 1\ncommand = [\"/bin/true\"]\n\n[capabilities]\n\"editor.execute\" = { commands = [\"mode.insert\"] }\n",
    )
    .unwrap();
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(app.plugins.allows_editor_command("compose", "mode.insert"));
    assert!(!app.plugins.allows_editor_command("compose", "mode.normal"));
    fs::remove_dir_all(root).unwrap();
}
