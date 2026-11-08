use std::fs;
use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;
use serde_json::json;

use super::focus::FocusTarget;
use super::terminal_ui::TerminalMode;
use super::window::FocusDirection;
use super::App;
use crate::keymap::Flavor;
use crate::test_support::temp_dir;

fn app() -> App {
    let mut app = App::new(Buffer::from_string("editor"), Flavor::Vim);
    app.set_screen_size(100, 30);
    app
}

fn open_terminal(app: &mut App, focus: bool, mode: TerminalMode) {
    app.terminal.set_test_state(true, mode);
    app.terminal_focus = focus;
}

fn open_explorer(app: &mut App, name: &str) -> PathBuf {
    let dir = temp_dir(name);
    fs::write(dir.join("file.txt"), "").unwrap();
    app.explorer.set_enabled(true);
    app.explorer.open(dir.clone());
    dir
}

fn cleanup(dir: &Path) {
    let _ = fs::remove_dir_all(dir);
}

fn press(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    app.handle_key(KeyEvent::new(code, modifiers)).expect("key");
}

#[test]
fn focus_target_clears_other_panels() {
    let mut app = app();
    let dir = open_explorer(&mut app, "focus_clear_panels");
    open_terminal(&mut app, true, TerminalMode::Input);
    app.toggle_debug_panel();
    app.terminal_focus = true;
    app.explorer.focus();
    let active = app.windows.active();

    assert!(app.focus_target(FocusTarget::Editor(active)));
    assert!(!app.terminal_focus);
    assert!(!app.debug_focus);
    assert!(!app.explorer.is_focused());
    assert_eq!(app.focused_target(), FocusTarget::Editor(active));
    cleanup(&dir);
}

#[test]
fn focus_direction_moves_between_editor_and_explorer() {
    let mut app = app();
    let dir = open_explorer(&mut app, "focus_editor_explorer");
    let active = app.windows.active();
    assert_eq!(app.focused_target(), FocusTarget::Explorer);
    assert!(app.focus_target(FocusTarget::Editor(active)));

    assert!(app.focus_direction(FocusDirection::Left));
    assert_eq!(app.focused_target(), FocusTarget::Explorer);

    assert!(app.focus_direction(FocusDirection::Right));
    assert_eq!(app.focused_target(), FocusTarget::Editor(active));
    cleanup(&dir);
}

#[test]
fn focus_direction_moves_between_editor_and_terminal() {
    let mut app = app();
    open_terminal(&mut app, false, TerminalMode::Input);
    let active = app.windows.active();

    assert!(app.focus_direction(FocusDirection::Down));
    assert_eq!(app.focused_target(), FocusTarget::Terminal);

    assert!(app.focus_direction(FocusDirection::Up));
    assert_eq!(app.focused_target(), FocusTarget::Editor(active));
}

#[test]
fn focus_direction_skips_hidden_debug_panel() {
    let mut app = app();
    let active = app.windows.active();
    assert!(!app.focus_direction(FocusDirection::Right));

    app.toggle_debug_panel();
    assert!(app.focus_target(FocusTarget::Editor(active)));
    assert!(app.focus_direction(FocusDirection::Right));
    assert_eq!(app.focused_target(), FocusTarget::Debug);

    app.close_debug_panel();
    assert!(app.focus_target(FocusTarget::Editor(active)));
    assert!(!app.focus_direction(FocusDirection::Right));
}

#[test]
fn focus_direction_reaches_plugin_panel() {
    let mut app = app();
    app.handle_plugin_request(
        "focus-test",
        "ui.panel_open",
        json!({
            "title": "Panel",
            "height": 6,
            "focus": true,
            "lines": [{"text": "line"}],
        }),
    )
    .unwrap();
    let active = app.windows.active();
    assert!(app.focus_target(FocusTarget::Editor(active)));

    assert!(app.focus_direction(FocusDirection::Down));
    assert_eq!(app.focused_target(), FocusTarget::PluginPanel);
}

#[test]
fn ctrl_w_moves_focus_to_explorer_and_back() {
    let mut app = app();
    let dir = open_explorer(&mut app, "focus_ctrl_w_explorer");
    let active = app.windows.active();
    assert!(app.focus_target(FocusTarget::Editor(active)));

    press(&mut app, KeyCode::Char('w'), KeyModifiers::CONTROL);
    press(&mut app, KeyCode::Char('h'), KeyModifiers::NONE);
    assert_eq!(app.focused_target(), FocusTarget::Explorer);

    press(&mut app, KeyCode::Char('w'), KeyModifiers::CONTROL);
    press(&mut app, KeyCode::Char('l'), KeyModifiers::NONE);
    assert_eq!(app.focused_target(), FocusTarget::Editor(active));
    cleanup(&dir);
}

#[test]
fn ctrl_w_is_forwarded_in_terminal_input_mode() {
    let mut app = app();
    open_terminal(&mut app, true, TerminalMode::Input);

    press(&mut app, KeyCode::Char('w'), KeyModifiers::CONTROL);
    assert!(!app.window_prefix);
    assert!(app.terminal_focus);
}

#[test]
fn ctrl_w_moves_focus_from_terminal_normal_mode() {
    let mut app = app();
    open_terminal(&mut app, true, TerminalMode::Normal);

    press(&mut app, KeyCode::Char('w'), KeyModifiers::CONTROL);
    assert!(app.window_prefix);
    press(&mut app, KeyCode::Char('k'), KeyModifiers::NONE);
    assert!(app.is_editor_focused());
    assert!(!app.terminal_focus);
}

#[test]
fn ctrl_w_ignores_window_commands_while_panel_focused() {
    let mut app = app();
    let dir = open_explorer(&mut app, "focus_ctrl_w_guard");
    assert_eq!(app.focused_target(), FocusTarget::Explorer);

    press(&mut app, KeyCode::Char('w'), KeyModifiers::CONTROL);
    press(&mut app, KeyCode::Char('s'), KeyModifiers::NONE);
    assert_eq!(app.windows.len(), 1);
    assert_eq!(app.focused_target(), FocusTarget::Explorer);
    cleanup(&dir);
}

#[test]
fn ctrl_w_w_returns_to_editor_from_panels() {
    let mut app = app();
    app.toggle_debug_panel();
    assert_eq!(app.focused_target(), FocusTarget::Debug);

    press(&mut app, KeyCode::Char('w'), KeyModifiers::CONTROL);
    press(&mut app, KeyCode::Char('w'), KeyModifiers::NONE);
    assert!(app.is_editor_focused());
    assert!(app.debug_panel_rect().is_some());

    app.handle_plugin_request(
        "focus-test",
        "ui.panel_open",
        json!({
            "title": "Panel",
            "height": 6,
            "focus": true,
            "lines": [{"text": "line"}],
        }),
    )
    .unwrap();
    assert_eq!(app.focused_target(), FocusTarget::PluginPanel);

    press(&mut app, KeyCode::Char('w'), KeyModifiers::CONTROL);
    press(&mut app, KeyCode::Char('w'), KeyModifiers::NONE);
    assert!(app.is_editor_focused());
    assert!(app.plugin_panel_view().is_some());
}

#[test]
fn focus_commands_are_registered() {
    let app = app();
    for name in [
        "focus.left",
        "focus.right",
        "focus.up",
        "focus.down",
        "focus.editor",
    ] {
        assert!(app.commands.find(name).is_some(), "missing {name}");
    }
}
