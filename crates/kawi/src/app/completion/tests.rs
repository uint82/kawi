use super::config::CompletionKeys;
use super::prefix::completion_prefix;
use super::{CompletionOptions, CompletionSession, KeyBinding};
use crate::app::App;
use crate::keymap::Flavor;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;
use kawi_completion::{Event, Trigger};
use std::path::PathBuf;
use std::time::Duration;

#[test]
fn completion_prefix_stops_at_non_identifier_characters() {
    let mut editor = kawi_editor::Editor::new(Buffer::from_string("value->field "));
    editor.apply(kawi_editor::Action::JumpTo(0, 12));
    let prefix = completion_prefix(&editor);
    assert_eq!(prefix.text, "field");
    assert_eq!(prefix.start, 7);
    assert_eq!(prefix.cursor, 12);
}

#[test]
fn manual_completion_reports_missing_names() {
    let mut app = App::new(Buffer::from_string("one"), Flavor::Vim);
    app.request_completion().expect("completion");
    assert_eq!(app.last_message(), Some("E32: No file name"));
}

#[test]
fn accepting_completion_cancels_a_scheduled_auto_trigger() {
    let mut app = App::new(Buffer::from_string("value\n"), Flavor::Vim);
    app.completion = Some(CompletionSession::new(
        Event {
            request: 1,
            path: PathBuf::from("/tmp/completion.rs"),
            version: 1,
            original_hash: 0,
            cursor: 0,
            word_start: 0,
            prefix: String::new(),
            items: Vec::new(),
        },
        "",
    ));
    app.completion_auto
        .schedule(Trigger::typing(), Duration::from_secs(1));
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        app.handle_completion_key(key).expect("completion"),
        Some(false)
    );
    assert!(app.completion_auto.wait_timeout().is_none());
}

#[test]
fn auto_completion_is_disabled_by_default() {
    assert!(!CompletionOptions::default().auto);
}

#[test]
fn scroll_bindings_default_to_ctrl_f_and_ctrl_b() {
    let keys = CompletionKeys::default();
    assert_eq!(keys.up, KeyBinding::control('b'));
    assert_eq!(keys.down, KeyBinding::control('f'));
}
