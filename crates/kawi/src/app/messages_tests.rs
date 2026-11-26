use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;
use kawi_tui::frame::MessageSeverity;

use super::App;
use crate::keymap::Flavor;

fn app() -> App {
    App::new(Buffer::from_string("alpha\nbeta\n"), Flavor::Vim)
}

fn execute(app: &mut App, text: &str) {
    app.execute_command_text(text).expect("execute");
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn messages_pages_history() {
    let mut app = app();
    execute(&mut app, "hlep");
    execute(&mut app, "messages");
    let lines = app.pager_lines().expect("pager");
    assert!(lines
        .iter()
        .any(|line| line.contains("E492: Not an editor command: hlep")));
    assert_eq!(app.last_message(), None);
}

#[test]
fn messages_clear_clears_history_and_current_message() {
    let mut app = app();
    execute(&mut app, "hlep");
    execute(&mut app, "messages clear");
    assert_eq!(app.last_message(), None);
    assert!(app.pager_lines().is_none());
    execute(&mut app, "messages");
    assert_eq!(app.last_message(), Some("No messages"));
}

#[test]
fn escape_keeps_history() {
    let mut app = app();
    execute(&mut app, "hlep");
    app.handle_key(key(KeyCode::Esc)).expect("escape");
    assert_eq!(app.last_message(), None);
    execute(&mut app, "messages");
    let lines = app.pager_lines().expect("pager");
    assert!(lines
        .iter()
        .any(|line| line.contains("E492: Not an editor command: hlep")));
}

#[test]
fn reports_do_not_enter_history() {
    let mut app = app();
    execute(&mut app, "ls");
    execute(&mut app, "messages");
    assert_eq!(app.last_message(), Some("No messages"));
}

#[test]
fn errors_are_errors_and_success_is_success() {
    let mut app = app();
    execute(&mut app, "hlep");
    assert_eq!(app.last_message_severity(), MessageSeverity::Error);
    execute(&mut app, "nmap gx edit.delete_line");
    assert_eq!(app.last_message_severity(), MessageSeverity::Success);
}
