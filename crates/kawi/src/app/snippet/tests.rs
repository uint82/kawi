use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;
use kawi_completion::Snippet;
use kawi_editor::Action;

use super::super::App;
use crate::keymap::Flavor;

fn app_at_end_of(text: &str) -> App {
    let mut app = App::new(Buffer::from_string(text), Flavor::Vim);
    app.editor.apply(Action::AppendLineEnd);
    app
}

fn tab() -> KeyEvent {
    KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)
}

fn shift_tab() -> KeyEvent {
    KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT)
}

fn press(app: &mut App, key: KeyEvent) {
    app.handle_key(key).expect("key");
}

#[test]
fn starting_a_snippet_selects_the_first_stop() {
    let mut app = app_at_end_of("value");
    app.start_snippet(Snippet::parse("${1:value}$0"));
    assert_eq!(app.editor.selection_range(), Some(((0, 0), (0, 5))));
}

#[test]
fn tab_walks_stops_then_finishes_at_the_final_position() {
    let mut app = app_at_end_of("one two");
    app.start_snippet(Snippet::parse("${1:one} ${2:two}"));
    assert_eq!(app.editor.selection_range(), Some(((0, 0), (0, 3))));
    press(&mut app, tab());
    assert_eq!(app.editor.selection_range(), Some(((0, 4), (0, 7))));
    press(&mut app, tab());
    assert!(app.snippet.is_none());
    assert_eq!(app.editor.cursor_char(), 7);
    assert!(app.editor.selection_range().is_none());
}

#[test]
fn shift_tab_walks_backwards() {
    let mut app = app_at_end_of("one two");
    app.start_snippet(Snippet::parse("${1:one} ${2:two}"));
    press(&mut app, tab());
    press(&mut app, shift_tab());
    assert_eq!(app.editor.selection_range(), Some(((0, 0), (0, 3))));
}

#[test]
fn shift_tab_at_the_first_stop_does_nothing() {
    let mut app = app_at_end_of("value");
    app.start_snippet(Snippet::parse("${1:value}$0"));
    press(&mut app, shift_tab());
    assert_eq!(app.editor.selection_range(), Some(((0, 0), (0, 5))));
    assert!(app.snippet.is_some());
}

#[test]
fn typing_replaces_a_stop_and_shifts_later_stops() {
    let mut app = app_at_end_of("one two");
    app.start_snippet(Snippet::parse("${1:one} ${2:two}"));
    press(
        &mut app,
        KeyEvent::new(KeyCode::Char('X'), KeyModifiers::NONE),
    );
    assert_eq!(app.editor.visible_lines(0, 1), ["X two"]);
    press(&mut app, tab());
    assert_eq!(app.editor.selection_range(), Some(((0, 2), (0, 5))));
}

#[test]
fn escape_ends_the_snippet_and_clears_the_selection() {
    let mut app = app_at_end_of("value");
    app.start_snippet(Snippet::parse("${1:value}$0"));
    press(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(app.snippet.is_none());
    assert!(app.editor.selection_range().is_none());
}

#[test]
fn navigation_ends_the_snippet() {
    let mut app = app_at_end_of("value");
    app.start_snippet(Snippet::parse("${1:value}$0"));
    press(&mut app, KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    assert!(app.snippet.is_none());
}

#[test]
fn snippets_without_stops_move_to_the_final_position() {
    let mut app = app_at_end_of("abcdef");
    app.start_snippet(Snippet::parse("abc$0def"));
    assert!(app.snippet.is_none());
    assert_eq!(app.editor.cursor_char(), 3);
    assert!(app.editor.selection_range().is_none());
}
