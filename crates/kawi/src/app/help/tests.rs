use super::*;
use crate::keymap::Flavor;
use crossterm::event::KeyEvent;
use kawi_buffer::Buffer;

fn app() -> App {
    App::new(Buffer::from_string("value\n"), Flavor::Vim)
}

fn execute(app: &mut App, command: &str) {
    app.run_command(kawi_editor::parse(command))
        .expect("command");
}

#[test]
fn help_opens_an_internal_readonly_buffer() {
    let mut app = app();
    execute(&mut app, "help diagnostics");
    assert!(app.editor.current_buffer_is_internal());
    assert!(app.editor.current_buffer_is_readonly());
    assert!(app.pager.is_none());
    let text = app
        .editor
        .buffer_text(app.editor.buffer_number())
        .expect("help");
    assert!(text.contains("Diagnostics"));
    assert!(text.contains("diagnostic_virtual_text"));
    assert!(text.contains("See also"));
}

#[test]
fn help_buffers_are_unlisted_but_remain_openable() {
    let mut app = app();
    execute(&mut app, "help");
    let number = app.editor.buffer_number();
    assert!(!app.editor.buffer_numbers().contains(&number));
    assert!(app.editor.all_buffer_numbers().contains(&number));
    assert!(app.editor.buffer_is_internal(number).unwrap_or(false));
}

#[test]
fn help_navigation_reuses_the_same_buffer_and_window() {
    let mut app = app();
    execute(&mut app, "help index");
    let number = app.editor.buffer_number();
    let windows = app.windows.len();
    execute(&mut app, "help diagnostics");
    assert_eq!(app.editor.buffer_number(), number);
    assert_eq!(app.windows.len(), windows);
}

#[test]
fn help_buffers_are_not_saved_in_workspace_sessions() {
    let mut app = app();
    execute(&mut app, "help");
    let session = app.session_snapshot().expect("session");
    assert!(!session
        .buffers
        .iter()
        .any(|buffer| buffer.path.as_deref() == Some("help:index")));
}

#[test]
fn help_resolves_aliases_and_reports_missing_topics() {
    let mut app = app();
    execute(&mut app, "help start");
    let text = app
        .editor
        .buffer_text(app.editor.buffer_number())
        .expect("help");
    assert!(text.contains("Getting started"));
    execute(&mut app, "help missing-topic");
    assert_eq!(
        app.last_message(),
        Some("E149: Sorry, no help for missing-topic")
    );
}

#[test]
fn help_links_follow_and_return() {
    let mut app = app();
    execute(&mut app, "help index");
    let row = app
        .editor
        .text_lines()
        .iter()
        .position(|line| line.contains("Getting started"))
        .expect("link row");
    app.editor.apply(kawi_editor::Action::JumpTo(row, 2));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .expect("follow");
    let text = app
        .editor
        .buffer_text(app.editor.buffer_number())
        .expect("help");
    assert!(text.contains("Getting started"));
    app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL))
        .expect("back");
    let text = app
        .editor
        .buffer_text(app.editor.buffer_number())
        .expect("help");
    assert!(text.contains("Kawi Help"));
}

#[test]
fn generated_option_help_shows_default_and_current_value() {
    let mut app = app();
    execute(&mut app, "help option diagnostic_virtual_text");
    let text = app
        .editor
        .buffer_text(app.editor.buffer_number())
        .expect("help");
    assert!(text.contains("Option diagnostic_virtual_text"));
    assert!(text.contains("default: all"));
    assert!(text.contains("current: all"));
}

#[test]
fn help_buffers_reject_writes() {
    let mut app = app();
    execute(&mut app, "help");
    app.run_command(kawi_editor::parse("w!")).expect("write");
    assert_eq!(
        app.last_message(),
        Some("E382: Cannot write, internal buffer")
    );
}

#[test]
fn help_option_alias_opens_the_option_reference() {
    let mut app = app();
    execute(&mut app, "help option");
    let text = app
        .editor
        .buffer_text(app.editor.buffer_number())
        .expect("help");
    assert!(text.contains("Options"));
    assert!(text.contains("diagnostic_virtual_text"));
}

#[test]
fn generated_option_help_shows_description_and_values() {
    let mut app = app();
    execute(&mut app, "help option signcolumn");
    let text = app
        .editor
        .buffer_text(app.editor.buffer_number())
        .expect("help");
    assert!(text.contains("Gutter sign column visibility."));
    assert!(text.contains("values: never, auto, always"));
}

#[test]
fn themes_help_lists_builtin_and_custom_discovery() {
    let mut app = app();
    execute(&mut app, "help themes");
    let text = app
        .editor
        .buffer_text(app.editor.buffer_number())
        .expect("help");
    assert!(text.contains("gruvbox-dark"));
    assert!(text.contains("~/.config/kawi/themes"));
    assert!(text.contains("Press Tab"));
}
