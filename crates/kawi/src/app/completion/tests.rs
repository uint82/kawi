use super::config::CompletionKeys;
use super::prefix::completion_prefix;
use super::{CompletionOptions, CompletionSession};
use crate::app::key_binding::KeyBinding;
use crate::app::App;
use crate::keymap::Flavor;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::{content_hash, Buffer, TextEdit};
use kawi_completion::{Event, Item, Snippet, Trigger};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

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

#[test]
fn accepting_a_snippet_item_starts_its_session() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kawi_snippet_accept_{stamp}"));
    fs::create_dir_all(&root).expect("project");
    let path = root.join("main.rs");
    let original = "pri\n";
    fs::write(&path, original).expect("file");
    let mut app = App::new(
        Buffer::from_file(path.to_str().expect("path")).expect("buffer"),
        Flavor::Vim,
    );
    app.editor.apply(kawi_editor::Action::AppendLineEnd);
    let session = CompletionSession::new(
        Event {
            request: 1,
            path: path.clone(),
            version: 1,
            original_hash: content_hash(original.as_bytes()),
            cursor: 3,
            word_start: 0,
            prefix: "pri".to_string(),
            items: vec![Item {
                source: "fake".to_string(),
                id: "print".to_string(),
                label: "print".to_string(),
                kind: None,
                filter_text: None,
                sort_text: None,
                detail: None,
                documentation: None,
                deprecated: false,
                preselect: false,
                snippet: Some(Snippet::parse("print!(${1:value})")),
                resolve: false,
                primary: TextEdit {
                    start: 0,
                    end: 3,
                    text: "print!(value)".to_string(),
                },
                additional: Vec::new(),
                command: None,
            }],
        },
        "pri",
    );
    app.accept_completion(&session);
    assert_eq!(app.editor.visible_lines(0, 1), ["print!(value)"]);
    assert_eq!(app.editor.selection_range(), Some(((0, 7), (0, 12))));
    assert!(app.snippet.is_some());
    fs::remove_dir_all(root).expect("cleanup");
}
