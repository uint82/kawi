use std::fs;

use kawi_plugin::{
    PluginHostEvent, PluginStatus, RegisteredCommand, RegisteredKeymap, INVALID_PARAMS,
};
use serde_json::json;

use super::common::*;
use crate::command::CommandHandler;

fn loaded() -> (std::path::PathBuf, crate::app::App) {
    let root = temp_dir("extensions");
    manifest(&root);
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    (root, app)
}

fn register(app: &mut crate::app::App) -> RegisteredCommand {
    let value = app
        .handle_plugin_request(
            "echo",
            "command.register",
            json!({
                "id": "greet",
                "title": "Greet",
                "description": "Say hello",
                "aliases": ["hello"]
            }),
        )
        .expect("register");
    serde_json::from_value(value).expect("registered command")
}

#[test]
fn registers_runtime_commands_with_aliases_and_args() {
    let (root, mut app) = loaded();
    let registered = register(&mut app);
    assert_eq!(registered.name, "echo.greet");
    assert_eq!(registered.description, "Say hello");
    let id = app.commands.find("echo.greet").expect("command");
    assert_eq!(app.commands.find("hello"), Some(id));
    let Some(CommandHandler::Plugin(target)) = app.commands.spec(id).map(|spec| spec.handler)
    else {
        panic!("plugin target");
    };
    assert!(app.plugins.target_accepts_args(target));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn lists_and_unregisters_runtime_commands() {
    let (root, mut app) = loaded();
    register(&mut app);
    set_keymap(&mut app, "normal", "<leader>pg", "hello");
    let mode = crate::keymap::KeyContext::Normal;
    let keys = crate::keymap::KeySequence::parse("<leader>pg", &app.leaders).unwrap();
    assert!(app.keymaps.binding_at(mode, keys.chords()).is_some());
    let value = app
        .handle_plugin_request("echo", "command.list", json!({}))
        .expect("list");
    let commands: Vec<RegisteredCommand> = serde_json::from_value(value).expect("commands");
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].id, "greet");
    app.handle_plugin_request("echo", "command.unregister", json!({ "id": "greet" }))
        .expect("unregister");
    assert!(app.commands.find("echo.greet").is_none());
    assert!(app.commands.find("hello").is_none());
    assert!(app.keymaps.binding_at(mode, keys.chords()).is_none());
    let error = app
        .handle_plugin_request("echo", "command.unregister", json!({ "id": "greet" }))
        .expect_err("unregistered");
    assert_eq!(error.code, INVALID_PARAMS);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn reuses_target_slots_after_unregistering_a_command() {
    let (root, mut app) = loaded();
    register(&mut app);
    let before = app.plugins.targets.len();
    app.handle_plugin_request("echo", "command.unregister", json!({ "id": "greet" }))
        .expect("unregister");
    let registered = register(&mut app);
    assert_eq!(app.plugins.targets.len(), before);
    let id = app.commands.find(&registered.name).expect("command");
    let Some(CommandHandler::Plugin(target)) = app.commands.spec(id).map(|spec| spec.handler)
    else {
        panic!("plugin target");
    };
    assert_eq!(app.plugins.target(target), Some(("echo", "greet")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_duplicate_and_reserved_runtime_commands() {
    let (root, mut app) = loaded();
    register(&mut app);
    let error = app
        .handle_plugin_request(
            "echo",
            "command.register",
            json!({
                "id": "greet",
                "title": "Again",
                "aliases": []
            }),
        )
        .expect_err("duplicate");
    assert_eq!(error.code, INVALID_PARAMS);
    let error = app
        .handle_plugin_request(
            "echo",
            "command.register",
            json!({
                "id": "ping",
                "title": "Shadow",
                "aliases": []
            }),
        )
        .expect_err("manifest command");
    assert_eq!(error.code, INVALID_PARAMS);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn stopping_a_plugin_removes_runtime_commands() {
    let (root, mut app) = loaded();
    register(&mut app);
    app.handle_plugin_event(PluginHostEvent::StatusChanged {
        plugin: "echo".to_string(),
        status: PluginStatus::Stopped,
    });
    assert!(app.commands.find("echo.greet").is_none());
    assert!(app.commands.find("hello").is_none());
    assert!(app.plugins.registered_commands("echo").is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn registers_lists_replaces_and_removes_runtime_keymaps() {
    let (root, mut app) = loaded();
    let registered = set_keymap(&mut app, "normal", "<leader>pg", "echo.ping");
    assert_eq!(registered.keys, "<leader>pg");
    let mode = crate::keymap::KeyContext::Normal;
    let keys = crate::keymap::KeySequence::parse("<leader>pg", &app.leaders).unwrap();
    let binding = app
        .keymaps
        .binding_at(mode, keys.chords())
        .expect("binding");
    assert_eq!(binding.source, crate::keymap::BindingSource::Plugin);
    let value = app
        .handle_plugin_request("echo", "keymap.list", json!({}))
        .expect("list");
    let keymaps: Vec<RegisteredKeymap> = serde_json::from_value(value).expect("keymaps");
    assert_eq!(keymaps.len(), 1);
    set_keymap(&mut app, "normal", "<leader>pg", "echo.ping");
    let value = app
        .handle_plugin_request("echo", "keymap.list", json!({}))
        .expect("list");
    let keymaps: Vec<RegisteredKeymap> = serde_json::from_value(value).expect("keymaps");
    assert_eq!(keymaps.len(), 1);
    let aliases = crate::keymap::KeySequence::parse("<leader>pg", &app.leaders).unwrap();
    app.handle_plugin_request(
        "echo",
        "keymap.remove",
        json!({ "mode": "normal", "keys": "<leader>pg" }),
    )
    .expect("remove");
    assert!(app.keymaps.binding_at(mode, aliases.chords()).is_none());
    assert!(app.plugins.registered_keymaps("echo").is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_runtime_keymap_collisions() {
    let (root, mut app) = loaded();
    let user_command = app.commands.find("file.save").expect("builtin");
    let mode = crate::keymap::KeyContext::Normal;
    let keys = crate::keymap::KeySequence::parse("<leader>pg", &app.leaders).unwrap();
    app.keymaps.bind(crate::keymap::Binding::new(
        user_command,
        mode,
        keys.clone(),
        crate::keymap::BindingSource::User,
    ));
    let error = app
        .handle_plugin_request(
            "echo",
            "keymap.set",
            json!({
                "mode": "normal",
                "keys": "<leader>pg",
                "command": "echo.ping"
            }),
        )
        .expect_err("user binding");
    assert_eq!(error.code, INVALID_PARAMS);
    app.keymaps.unbind(mode, &keys);
    let target = app.plugins.targets.len() as u32;
    app.commands
        .register_plugin(crate::command::PluginCommandRegistration {
            name: "other.ping".to_string(),
            aliases: Vec::new(),
            description: "Ping".to_string(),
            plugin: "other".to_string(),
            target,
        })
        .unwrap();
    app.plugins
        .targets
        .push(Some(crate::app::plugin::runtime::PluginCommandTarget {
            plugin: "other".to_string(),
            command: "ping".to_string(),
            required: Vec::new(),
            coalesce: false,
            accepts_args: false,
        }));
    let other = app.commands.find("other.ping").expect("other");
    app.keymaps.bind(crate::keymap::Binding::new(
        other,
        mode,
        keys.clone(),
        crate::keymap::BindingSource::Plugin,
    ));
    let error = app
        .handle_plugin_request(
            "echo",
            "keymap.set",
            json!({
                "mode": "normal",
                "keys": "<leader>pg",
                "command": "echo.ping"
            }),
        )
        .expect_err("plugin binding");
    assert_eq!(error.code, INVALID_PARAMS);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_runtime_keymaps_with_reserved_prefixes() {
    let (root, mut app) = loaded();
    for keys in ["1", "\"a"] {
        let error = app
            .handle_plugin_request(
                "echo",
                "keymap.set",
                json!({
                    "mode": "normal",
                    "keys": keys,
                    "command": "echo.ping"
                }),
            )
            .expect_err("reserved prefix");
        assert_eq!(error.code, INVALID_PARAMS);
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_runtime_keymaps_that_shadow_a_user_prefix() {
    let (root, mut app) = loaded();
    let user_command = app.commands.find("file.save").expect("builtin");
    let mode = crate::keymap::KeyContext::Normal;
    let user_keys = crate::keymap::KeySequence::parse("<leader>pg", &app.leaders).unwrap();
    app.keymaps.bind(crate::keymap::Binding::new(
        user_command,
        mode,
        user_keys,
        crate::keymap::BindingSource::User,
    ));
    let error = app
        .handle_plugin_request(
            "echo",
            "keymap.set",
            json!({
                "mode": "normal",
                "keys": "<leader>p",
                "command": "echo.ping"
            }),
        )
        .expect_err("shorter than user mapping");
    assert_eq!(error.code, INVALID_PARAMS);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_runtime_keymaps_that_extend_a_user_prefix() {
    let (root, mut app) = loaded();
    let user_command = app.commands.find("file.save").expect("builtin");
    let mode = crate::keymap::KeyContext::Normal;
    let user_keys = crate::keymap::KeySequence::parse("<leader>p", &app.leaders).unwrap();
    app.keymaps.bind(crate::keymap::Binding::new(
        user_command,
        mode,
        user_keys,
        crate::keymap::BindingSource::User,
    ));
    let error = app
        .handle_plugin_request(
            "echo",
            "keymap.set",
            json!({
                "mode": "normal",
                "keys": "<leader>pg",
                "command": "echo.ping"
            }),
        )
        .expect_err("longer than user mapping");
    assert_eq!(error.code, INVALID_PARAMS);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn stopping_a_plugin_removes_runtime_keymaps() {
    let (root, mut app) = loaded();
    set_keymap(&mut app, "normal", "<leader>pg", "echo.ping");
    app.handle_plugin_event(PluginHostEvent::StatusChanged {
        plugin: "echo".to_string(),
        status: PluginStatus::Stopped,
    });
    let mode = crate::keymap::KeyContext::Normal;
    let keys = crate::keymap::KeySequence::parse("<leader>pg", &app.leaders).unwrap();
    assert!(app.keymaps.binding_at(mode, keys.chords()).is_none());
    assert!(app.plugins.registered_keymaps("echo").is_empty());
    fs::remove_dir_all(root).unwrap();
}

fn set_keymap(
    app: &mut crate::app::App,
    mode: &str,
    keys: &str,
    command: &str,
) -> RegisteredKeymap {
    let value = app
        .handle_plugin_request(
            "echo",
            "keymap.set",
            json!({ "mode": mode, "keys": keys, "command": command }),
        )
        .expect("set keymap");
    serde_json::from_value(value).expect("keymap")
}

#[test]
fn skips_manifest_keybindings_that_shadow_a_user_prefix() {
    let root = temp_dir("manifest_user_prefix");
    write_manifest(
        &root,
        "echo",
        "0.1.0",
        r#"
[[commands]]
id = "ping"
title = "Ping"

[commands.keybindings]
normal = ["<leader>p"]
"#,
    );
    let mut app = app();
    let user_command = app.commands.find("file.save").expect("builtin");
    let mode = crate::keymap::KeyContext::Normal;
    let user_keys = crate::keymap::KeySequence::parse("<leader>pg", &app.leaders).unwrap();
    app.keymaps.bind(crate::keymap::Binding::new(
        user_command,
        mode,
        user_keys.clone(),
        crate::keymap::BindingSource::User,
    ));
    let warnings = load_plugins(&mut app, &root);
    assert!(
        warnings
            .iter()
            .any(|warning| warning.contains("conflicts with a user mapping")),
        "{warnings:?}"
    );
    let plugin_keys = crate::keymap::KeySequence::parse("<leader>p", &app.leaders).unwrap();
    assert!(app.keymaps.binding_at(mode, plugin_keys.chords()).is_none());
    assert_eq!(
        app.keymaps
            .binding_at(mode, user_keys.chords())
            .map(|binding| binding.source),
        Some(crate::keymap::BindingSource::User)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn user_prefix_wins_when_keymaps_reload() {
    let (root, mut app) = loaded();
    let user_command = app.commands.find("file.save").expect("builtin");
    let mode = crate::keymap::KeyContext::Normal;
    let user_keys = crate::keymap::KeySequence::parse("<leader>p", &app.leaders).unwrap();
    let mut load = crate::keymap::config::KeymapLoad::default();
    load.bindings.push(crate::keymap::Binding::new(
        user_command,
        mode,
        user_keys.clone(),
        crate::keymap::BindingSource::User,
    ));
    app.apply_keymaps(load);
    let plugin_keys = crate::keymap::KeySequence::parse("<leader>pp", &app.leaders).unwrap();
    assert!(app.keymaps.binding_at(mode, plugin_keys.chords()).is_none());
    assert_eq!(
        app.keymaps
            .binding_at(mode, user_keys.chords())
            .map(|binding| binding.source),
        Some(crate::keymap::BindingSource::User)
    );
    fs::remove_dir_all(root).unwrap();
}
