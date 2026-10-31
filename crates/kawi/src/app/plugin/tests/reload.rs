use std::fs;

use super::common::*;
use crate::app::App;
use crate::command::CommandHandler;
use crate::keymap::BindingSource;

struct ManifestSpec<'a> {
    name: &'a str,
    command: &'a str,
    keys: &'a str,
    version: &'a str,
}

fn write_manifest(root: &std::path::Path, spec: ManifestSpec<'_>) {
    let dir = root.join(spec.name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        format!(
            r#"name = "{}"
version = "{}"
protocol = 1
command = ["/bin/true"]

[[commands]]
id = "{}"
title = "Ping"
description = "Return a greeting"

[commands.keybindings]
normal = ["{}"]
"#,
            spec.name, spec.version, spec.command, spec.keys
        ),
    )
    .unwrap();
}

fn plugin_target(app: &App, name: &str) -> Option<(String, String)> {
    let command = app.commands.find(name)?;
    match app.commands.spec(command)?.handler {
        CommandHandler::Plugin(target) => app
            .plugins
            .target(target)
            .map(|(plugin, command)| (plugin.to_string(), command.to_string())),
        _ => None,
    }
}

#[test]
fn restart_reloads_manifest_commands_and_bindings() {
    let root = temp_dir("reload_manifest");
    write_manifest(
        &root,
        ManifestSpec {
            name: "echo",
            command: "ping",
            keys: "<leader>pp",
            version: "0.1.0",
        },
    );
    let mut app = app();
    load_plugins(&mut app, &root);
    let old = app.commands.find("echo.ping").expect("old command");

    write_manifest(
        &root,
        ManifestSpec {
            name: "echo",
            command: "hello",
            keys: "<leader>ph",
            version: "0.2.0",
        },
    );
    app.handle_plugin_command("restart");

    assert_eq!(app.message.as_deref(), Some("Plugins reloaded"));
    assert!(app.commands.find("echo.ping").is_none());
    let command = app.commands.find("echo.hello").expect("new command");
    assert_eq!(
        plugin_target(&app, "echo.hello"),
        Some(("echo".to_string(), "hello".to_string()))
    );
    assert!(app
        .keymaps
        .bindings()
        .iter()
        .all(|binding| { binding.source != BindingSource::Plugin || binding.command != old }));
    assert!(app
        .keymaps
        .bindings()
        .iter()
        .filter(|binding| binding.source == BindingSource::Plugin)
        .all(|binding| binding.command == command));
    assert_eq!(app.plugins.plugins()[0].manifest.version, "0.2.0");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn named_restart_reloads_only_that_plugin() {
    let root = temp_dir("reload_named");
    write_manifest(
        &root,
        ManifestSpec {
            name: "echo",
            command: "ping",
            keys: "<leader>pp",
            version: "0.1.0",
        },
    );
    write_manifest(
        &root,
        ManifestSpec {
            name: "other",
            command: "ping",
            keys: "<leader>op",
            version: "0.1.0",
        },
    );
    let mut app = app();
    load_plugins(&mut app, &root);

    write_manifest(
        &root,
        ManifestSpec {
            name: "echo",
            command: "hello",
            keys: "<leader>ph",
            version: "0.2.0",
        },
    );
    write_manifest(
        &root,
        ManifestSpec {
            name: "other",
            command: "changed",
            keys: "<leader>oc",
            version: "0.2.0",
        },
    );
    app.handle_plugin_command("restart echo");

    assert_eq!(app.message.as_deref(), Some("Plugin reloaded: echo"));
    assert_eq!(
        plugin_target(&app, "echo.hello"),
        Some(("echo".to_string(), "hello".to_string()))
    );
    assert_eq!(
        plugin_target(&app, "other.ping"),
        Some(("other".to_string(), "ping".to_string()))
    );
    assert!(app.commands.find("other.changed").is_none());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn restart_keeps_user_bindings_for_reloaded_commands() {
    let root = temp_dir("reload_user_binding");
    write_manifest(
        &root,
        ManifestSpec {
            name: "echo",
            command: "ping",
            keys: "<leader>pp",
            version: "0.1.0",
        },
    );
    let mut app = app();
    load_plugins(&mut app, &root);
    let command = app.commands.find("echo.ping").expect("command");
    app.handle_map_command("nmap zz echo.ping").unwrap();

    write_manifest(
        &root,
        ManifestSpec {
            name: "echo",
            command: "ping",
            keys: "<leader>pq",
            version: "0.2.0",
        },
    );
    app.handle_plugin_command("restart");

    assert_eq!(app.commands.find("echo.ping"), Some(command));
    assert!(app.keymaps.bindings().iter().any(|binding| {
        binding.command == command
            && binding.source == BindingSource::User
            && binding.keys.to_string() == "zz"
    }));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn restart_removes_deleted_plugins() {
    let root = temp_dir("reload_deleted");
    write_manifest(
        &root,
        ManifestSpec {
            name: "echo",
            command: "ping",
            keys: "<leader>pp",
            version: "0.1.0",
        },
    );
    let mut app = app();
    load_plugins(&mut app, &root);

    fs::remove_dir_all(root.join("echo")).unwrap();
    app.handle_plugin_command("restart echo");

    assert_eq!(app.message.as_deref(), Some("Plugin removed: echo"));
    assert!(app.commands.find("echo.ping").is_none());
    assert!(app.plugins.plugins().is_empty());
    assert!(app
        .keymaps
        .bindings()
        .iter()
        .all(|binding| binding.source != BindingSource::Plugin));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn restart_discovers_added_plugins() {
    let root = temp_dir("reload_added");
    write_manifest(
        &root,
        ManifestSpec {
            name: "echo",
            command: "ping",
            keys: "<leader>pp",
            version: "0.1.0",
        },
    );
    let mut app = app();
    load_plugins(&mut app, &root);

    write_manifest(
        &root,
        ManifestSpec {
            name: "extra",
            command: "ping",
            keys: "<leader>xp",
            version: "0.1.0",
        },
    );
    app.handle_plugin_command("restart");

    assert_eq!(
        plugin_target(&app, "extra.ping"),
        Some(("extra".to_string(), "ping".to_string()))
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn restart_starts_plugin_with_reloaded_manifest() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let root = temp_dir("reload_worker");
    let dir = copy_example_plugin(&root, "echo");
    let mut app = app();
    load_plugins(&mut app, &root);
    assert_eq!(
        invoke_plugin(&mut app, "echo.hello").as_deref(),
        Some("pong world")
    );

    let python = fs::read_to_string(dir.join("plugin.py")).unwrap();
    fs::write(
        dir.join("plugin.py"),
        python.replace("pong {name}", "reloaded world"),
    )
    .unwrap();
    app.handle_plugin_command("restart echo");
    assert_eq!(app.message.as_deref(), Some("Plugin reloaded: echo"));
    assert_eq!(
        invoke_plugin(&mut app, "echo.hello").as_deref(),
        Some("reloaded world")
    );
    fs::remove_dir_all(root).unwrap();
}
