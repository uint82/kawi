use std::fs;
use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use kawi_buffer::Buffer;
use serde_json::json;

use super::tests::key;
use super::{TerminalMode, TerminalUi};
use crate::app::key_binding::KeyBinding;
use crate::app::App;
use crate::keymap::{Flavor, Keymap};
use crate::test_support::temp_dir;

fn terminal_app(focus: bool, mode: TerminalMode) -> App {
    let mut app = App::new(Buffer::from_string("editor"), Flavor::Vim);
    app.set_screen_size(100, 30);
    app.terminal = TerminalUi {
        open: true,
        mode,
        ..TerminalUi::default()
    };
    app.terminal_focus = focus;
    app.terminal_focus_key = Some(KeyBinding::control('g'));
    app
}

fn focus_explorer(app: &mut App, name: &str) -> PathBuf {
    let dir = temp_dir(name);
    fs::write(dir.join("file.txt"), "").unwrap();
    app.explorer.set_enabled(true);
    app.explorer.open(dir.clone());
    dir
}

fn cleanup(dir: &Path) {
    let _ = fs::remove_dir_all(dir);
}

fn press_focus_key(app: &mut App) {
    app.handle_key(key(KeyCode::Char('g'), KeyModifiers::CONTROL))
        .unwrap();
}

#[test]
fn focus_key_is_unbound_by_default() {
    let mut app = terminal_app(false, TerminalMode::Input);
    app.terminal_focus_key = None;
    press_focus_key(&mut app);
    assert!(!app.terminal_focus);
}

#[test]
fn focus_key_focuses_terminal_from_editor() {
    let mut app = terminal_app(false, TerminalMode::Input);
    press_focus_key(&mut app);
    assert!(app.terminal_focus);
}

#[test]
fn focus_key_returns_to_editor_from_input_mode() {
    let mut app = terminal_app(true, TerminalMode::Input);
    press_focus_key(&mut app);
    assert!(!app.terminal_focus);
    assert!(app.terminal.is_open());
}

#[test]
fn focus_key_is_intercepted_before_terminal_normal_mode() {
    let mut app = terminal_app(true, TerminalMode::Normal);
    press_focus_key(&mut app);
    assert!(!app.terminal_focus);
}

#[test]
fn configured_focus_key_is_used() {
    let mut app = terminal_app(false, TerminalMode::Input);
    app.terminal_focus_key = Some(KeyBinding::control('k'));
    press_focus_key(&mut app);
    assert!(!app.terminal_focus);
    app.handle_key(key(KeyCode::Char('k'), KeyModifiers::CONTROL))
        .unwrap();
    assert!(app.terminal_focus);
}

#[test]
fn escape_in_normal_mode_returns_focus_to_editor() {
    let mut app = terminal_app(true, TerminalMode::Normal);
    app.handle_key(key(KeyCode::Esc, KeyModifiers::NONE))
        .unwrap();
    assert!(!app.terminal_focus);
}

#[test]
fn escape_chord_still_enters_terminal_normal_mode() {
    let mut app = terminal_app(true, TerminalMode::Input);
    app.handle_key(key(KeyCode::Char('\\'), KeyModifiers::CONTROL))
        .unwrap();
    app.handle_key(key(KeyCode::Char('n'), KeyModifiers::CONTROL))
        .unwrap();
    assert_eq!(app.terminal.mode, TerminalMode::Normal);
    assert!(app.terminal_focus);
}

#[test]
fn focus_key_releases_explorer_focus() {
    let mut app = terminal_app(false, TerminalMode::Input);
    let dir = focus_explorer(&mut app, "terminal_focus_explorer");
    assert!(app.explorer.is_focused());
    press_focus_key(&mut app);
    assert!(app.terminal_focus);
    assert!(!app.explorer.is_focused());
    assert!(app.explorer.is_visible());
    press_focus_key(&mut app);
    assert!(!app.terminal_focus);
    assert!(!app.explorer.is_focused());
    cleanup(&dir);
}

#[test]
fn focus_key_releases_debug_panel_focus() {
    let mut app = terminal_app(false, TerminalMode::Input);
    app.debug_focus = true;
    press_focus_key(&mut app);
    assert!(app.terminal_focus);
    assert!(!app.debug_focus);
}

#[test]
fn focus_key_releases_plugin_panel_focus() {
    let mut app = terminal_app(false, TerminalMode::Input);
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
    assert!(app.plugin_panel.is_focused());
    press_focus_key(&mut app);
    assert!(app.terminal_focus);
    assert!(!app.plugin_panel.is_focused());
}

#[test]
fn focus_key_cancels_window_prefix() {
    let mut app = terminal_app(false, TerminalMode::Input);
    app.handle_key(key(KeyCode::Char('w'), KeyModifiers::CONTROL))
        .unwrap();
    assert!(app.window_prefix);
    press_focus_key(&mut app);
    assert!(app.terminal_focus);
    assert!(!app.window_prefix);
}

#[test]
fn focus_key_cancels_leader_sequence() {
    let mut app = terminal_app(false, TerminalMode::Input);
    app.handle_key(key(KeyCode::Char(' '), KeyModifiers::NONE))
        .unwrap();
    assert!(app.keymap.leader_path().is_some());
    press_focus_key(&mut app);
    assert!(app.terminal_focus);
    assert!(app.keymap.leader_path().is_none());
}

#[test]
fn focus_key_defers_to_pager() {
    let mut app = terminal_app(false, TerminalMode::Input);
    app.pager = Some(vec!["message".to_string()]);
    press_focus_key(&mut app);
    assert!(!app.terminal_focus);
    assert!(app.pager.is_none());
}

#[test]
fn focus_key_returns_from_terminal_despite_overlay() {
    let mut app = terminal_app(true, TerminalMode::Input);
    app.pager = Some(vec!["message".to_string()]);
    press_focus_key(&mut app);
    assert!(!app.terminal_focus);
    assert!(app.pager.is_some());
}

#[test]
fn terminal_scroll_keeps_normal_mode() {
    let mut app = terminal_app(true, TerminalMode::Normal);
    let rect = app.terminal_panel_rect().expect("terminal panel");
    let scroll = MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: u16::try_from(rect.x).unwrap(),
        row: u16::try_from(rect.y).unwrap(),
        modifiers: KeyModifiers::NONE,
    };
    app.handle_mouse(scroll).unwrap();
    assert_eq!(app.terminal.mode, TerminalMode::Normal);
}

#[test]
fn terminal_mouse_focus_releases_explorer_focus() {
    let mut app = terminal_app(false, TerminalMode::Input);
    let dir = focus_explorer(&mut app, "terminal_mouse_explorer");
    let rect = app.terminal_panel_rect().expect("terminal panel");
    let click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: u16::try_from(rect.x).unwrap(),
        row: u16::try_from(rect.y).unwrap(),
        modifiers: KeyModifiers::NONE,
    };
    app.handle_mouse(click).unwrap();
    assert!(app.terminal_focus);
    assert!(!app.explorer.is_focused());
    cleanup(&dir);
}

#[test]
#[cfg(unix)]
fn focus_key_opens_closed_terminal_over_explorer() {
    let mut app = App::new(Buffer::from_string("editor"), Flavor::Vim);
    app.set_screen_size(100, 30);
    app.terminal_focus_key = Some(KeyBinding::control('g'));
    let dir = focus_explorer(&mut app, "terminal_focus_open_explorer");
    assert!(app.explorer.is_focused());
    press_focus_key(&mut app);
    assert!(app.terminal.is_open());
    assert!(app.terminal_focus);
    assert!(!app.explorer.is_focused());
    app.close_terminal();
    cleanup(&dir);
}
