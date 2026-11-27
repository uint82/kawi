use super::*;
use crate::keymap::Flavor;
use kawi_buffer::Buffer;
use std::fs;
use std::path::Path;

fn app() -> App {
    App::new(Buffer::from_string("value\n"), Flavor::Vim)
}

fn state_paths(root: &Path) -> StatePaths {
    StatePaths {
        config: root.join("config.toml"),
        config_log: root.join("config.log"),
        #[cfg(feature = "wasm")]
        cache: root.join("cache"),
        keymaps: root.join("keymaps.toml"),
        plugins: root.join("plugins"),
        installed_plugins: root.join("installed-plugins"),
        plugin_lock: root.join("plugins.lock"),
        plugin_settings: root.join("plugins.toml"),
        languages: root.join("languages.toml"),
        debug: root.join("debug.toml"),
        tasks: root.join("tasks.toml"),
        themes: root.join("themes"),
        queries: root.join("queries"),
        history: root.join("history"),
        sessions: root.join("sessions"),
        views: root.join("views"),
        recovery: root.join("recovery"),
        undo: root.join("undo"),
    }
}

#[test]
fn health_check_aliases_resolve_to_check_ids() {
    assert_eq!(health_check_id("lsp"), Some("languages"));
    assert_eq!(health_check_id("LSP"), Some("languages"));
    assert_eq!(health_check_id("debug"), Some("debug"));
    assert_eq!(
        health_check_id("plugin-keybindings"),
        Some("plugin_keybindings")
    );
    assert_eq!(health_check_id("unknown"), None);
}

#[test]
fn health_report_uses_the_health_title() {
    let app = app();
    let text = app.health_report(None, None);
    assert!(text.starts_with("Kawi health"));
    assert!(text.contains("[error] Paths"));
}

#[test]
fn health_report_filters_to_languages() {
    let dir = std::env::temp_dir().join(format!("kawi_health_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let paths = state_paths(&dir);
    let app = app();
    let text = app.health_report(Some("languages"), Some(&paths));
    assert!(text.contains("Languages"));
    assert!(!text.contains("Paths"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn health_command_opens_and_reuses_an_internal_buffer() {
    let mut app = app();
    app.execute_command_text("health").unwrap();
    assert!(app.editor.current_buffer_is_internal());
    let number = app.editor.buffer_number();
    let text = app.editor.buffer_text(number).expect("health buffer");
    assert!(text.starts_with("Kawi health"));
    let windows = app.windows.len();
    app.execute_command_text("health lsp").unwrap();
    assert_eq!(app.editor.buffer_number(), number);
    assert_eq!(app.windows.len(), windows);
}

#[test]
fn unknown_health_check_reports_an_error() {
    let mut app = app();
    app.execute_command_text("health nope").unwrap();
    assert_eq!(
        app.message.as_deref(),
        Some("E149: Sorry, no health check for nope")
    );
    assert!(!app.editor.current_buffer_is_internal());
}
