use crossterm::event::{KeyCode, KeyModifiers};
use kawi_buffer::Buffer;

use super::tests::key;
use super::{TerminalMode, TerminalUi};
use crate::app::key_binding::KeyBinding;
use crate::app::App;
use crate::keymap::Flavor;

fn terminal_app(focus: bool, mode: TerminalMode) -> App {
    let mut app = App::new(Buffer::from_string("editor"), Flavor::Vim);
    app.set_screen_size(100, 30);
    app.terminal = TerminalUi {
        open: true,
        mode,
        ..TerminalUi::default()
    };
    app.terminal_focus = focus;
    app
}

#[test]
fn focus_key_focuses_terminal_from_editor() {
    let mut app = terminal_app(false, TerminalMode::Input);
    app.handle_key(key(KeyCode::Char('g'), KeyModifiers::CONTROL))
        .unwrap();
    assert!(app.terminal_focus);
}

#[test]
fn focus_key_returns_to_editor_from_input_mode() {
    let mut app = terminal_app(true, TerminalMode::Input);
    app.handle_key(key(KeyCode::Char('g'), KeyModifiers::CONTROL))
        .unwrap();
    assert!(!app.terminal_focus);
    assert!(app.terminal.is_open());
}

#[test]
fn focus_key_is_intercepted_before_terminal_normal_mode() {
    let mut app = terminal_app(true, TerminalMode::Normal);
    app.handle_key(key(KeyCode::Char('g'), KeyModifiers::CONTROL))
        .unwrap();
    assert!(!app.terminal_focus);
}

#[test]
fn configured_focus_key_is_used() {
    let mut app = terminal_app(false, TerminalMode::Input);
    app.terminal_focus_key = KeyBinding::control('k');
    app.handle_key(key(KeyCode::Char('g'), KeyModifiers::CONTROL))
        .unwrap();
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
#[cfg(unix)]
fn focus_key_opens_closed_terminal() {
    let mut app = App::new(Buffer::from_string("editor"), Flavor::Vim);
    app.set_screen_size(100, 30);
    app.handle_key(key(KeyCode::Char('g'), KeyModifiers::CONTROL))
        .unwrap();
    assert!(app.terminal.is_open());
    assert!(app.terminal_focus);
    app.close_terminal();
}
