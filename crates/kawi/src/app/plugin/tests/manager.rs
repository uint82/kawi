use std::fs;
use std::path::Path;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;
use crate::plugin_manager::test_client::{client, client_at};

use super::common::{app, configure_plugin_manager, temp_dir};

fn confirm(app: &mut App) {
    app.handle_plugin_prompt_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
}

fn install(app: &mut App, root: &Path) {
    configure_plugin_manager(app, root);
    app.plugin_manager.set_client(Box::new(client("")));
    super::common::load_plugins(app, root);
    app.handle_plugin_command("install owner/repo");
    confirm(app);
}

fn status_text(app: &App) -> String {
    match (&app.message, &app.pager) {
        (Some(message), _) => message.clone(),
        (None, Some(lines)) => lines.join("\n"),
        (None, None) => String::new(),
    }
}

#[test]
fn install_requires_a_source() {
    let mut app = app();
    app.handle_plugin_command("install");
    assert_eq!(
        app.message.as_deref(),
        Some("Usage: plugin install <owner/repo[@ref]>")
    );
}

#[test]
fn install_shows_one_confirmation_and_enables_the_plugin() {
    let root = temp_dir("manager-install");
    let mut app = app();
    configure_plugin_manager(&mut app, &root);
    app.plugin_manager.set_client(Box::new(client("")));
    super::common::load_plugins(&mut app, &root);
    app.handle_plugin_command("install owner/repo");
    assert!(app.is_plugin_prompt_active());
    let message = app.message.clone().expect("prompt message");
    assert!(message.contains("owner/repo"));
    let (lines, _) = app.plugin_prompt_items().expect("prompt lines");
    for expected in [
        "author: owner",
        "api: 1",
        "repository: owner/repo",
        "ui.message unscoped",
    ] {
        assert!(
            lines.iter().any(|line| line.contains(expected)),
            "missing {expected}"
        );
    }
    confirm(&mut app);
    assert!(!app.is_plugin_prompt_active());
    assert!(app.plugins.contains("echo"));
    assert!(app.plugin_manager.lock().plugins["echo"].enabled);
    app.plugins.shutdown();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn update_without_api_changes_applies_without_confirmation() {
    let root = temp_dir("manager-update-silent");
    let mut app = app();
    install(&mut app, &root);
    let commit = "1123456789abcdef0123456789abcdef01234567";
    app.plugin_manager
        .set_client(Box::new(client_at("", commit)));
    app.handle_plugin_command("update echo");
    assert!(!app.is_plugin_prompt_active());
    assert_eq!(
        app.plugin_manager.lock().plugins["echo"].commit.as_str(),
        commit
    );
    app.plugins.shutdown();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn update_with_new_host_apis_prompts_once() {
    let root = temp_dir("manager-update-prompt");
    let mut app = app();
    install(&mut app, &root);
    app.plugin_manager.set_client(Box::new(client_at(
        "\"editor.read\" = true\n",
        "1123456789abcdef0123456789abcdef01234567",
    )));
    app.handle_plugin_command("update echo");
    assert!(app.is_plugin_prompt_active());
    let (lines, _) = app.plugin_prompt_items().expect("prompt lines");
    assert!(lines.iter().any(|line| line == "editor.read"));
    confirm(&mut app);
    assert!(!app.is_plugin_prompt_active());
    assert!(app.plugin_manager.lock().plugins["echo"].enabled);
    app.plugins.shutdown();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn disable_and_enable_managed_plugin() {
    let root = temp_dir("manager-disable");
    let mut app = app();
    install(&mut app, &root);
    app.handle_plugin_command("disable echo");
    assert!(!app.plugin_manager.lock().plugins["echo"].enabled);
    assert!(!app.plugins.contains("echo"));
    app.handle_plugin_command("enable echo");
    assert!(app.plugin_manager.lock().plugins["echo"].enabled);
    assert!(app.plugins.contains("echo"));
    app.plugins.shutdown();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn development_plugins_are_not_managed() {
    let root = temp_dir("manager-dev");
    let mut app = app();
    configure_plugin_manager(&mut app, &root);
    super::common::manifest(&root);
    super::common::load_plugins(&mut app, &root);
    app.handle_plugin_command("disable echo");
    assert!(status_text(&app).contains("development plugin"));
    app.handle_plugin_command("remove echo");
    assert!(status_text(&app).contains("development plugin"));
    app.plugins.shutdown();
    fs::remove_dir_all(root).unwrap();
}
