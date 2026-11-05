use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;
use kawi_lsp::{CursorPosition, DocumentColor};

use super::super::App;
use crate::keymap::Flavor;

fn app() -> (App, PathBuf) {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kawi_color_picker_{stamp}"));
    fs::create_dir_all(&root).expect("directory");
    let path = root.join("theme.css");
    fs::write(&path, "#cc2233\n").expect("file");
    let app = App::new(
        Buffer::from_file(path.to_str().expect("path")).expect("buffer"),
        Flavor::Vim,
    );
    (app, path)
}

fn color() -> DocumentColor {
    DocumentColor {
        start: CursorPosition { line: 0, column: 0 },
        end: CursorPosition { line: 0, column: 7 },
        red: 204,
        green: 34,
        blue: 51,
        alpha: 255,
    }
}

#[test]
fn opens_only_when_a_server_color_is_under_the_cursor() {
    let (mut app, path) = app();
    let mut offset = color();
    offset.start.line = 1;
    offset.end.line = 1;
    app.lsp
        .set_color_preview_colors_for_test(&path, 0, vec![offset]);
    assert!(!app.open_color_picker().expect("open"));
    assert!(app.color_picker.is_none());
    app.lsp
        .set_color_preview_colors_for_test(&path, 0, vec![color()]);
    assert!(!app.open_color_picker().expect("open"));
    assert!(app.color_picker.is_some());
}

#[test]
fn color_command_opens_the_picker() {
    let (mut app, path) = app();
    app.lsp
        .set_color_preview_colors_for_test(&path, 0, vec![color()]);
    app.run_command(kawi_editor::parse("color"))
        .expect("command");
    assert!(app.color_picker.is_some());
}

#[test]
fn picker_keys_keep_the_editor_running() {
    let (mut app, path) = app();
    app.lsp
        .set_color_preview_colors_for_test(&path, 0, vec![color()]);
    app.open_color_picker().expect("open");
    let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    assert!(!app.handle_color_picker_key(key).expect("key"));
    assert!(app.color_picker.is_some());
}

#[test]
fn applies_a_selected_presentation() {
    let (mut app, path) = app();
    app.lsp
        .set_color_preview_colors_for_test(&path, 0, vec![color()]);
    app.open_color_picker().expect("open");
    let picker = app.color_picker.as_mut().expect("picker");
    picker.begin_request(1);
    assert!(picker.apply_event(kawi_lsp::ColorPresentationEvent {
        request: 1,
        path: path.clone(),
        presentations: vec![kawi_lsp::ColorPresentationEntry {
            label: "#112233".to_string(),
            edit: kawi_lsp::TextChange {
                start: 0,
                end: 7,
                text: "#112233".to_string(),
            },
            additional: Vec::new(),
        }],
    }));
    app.apply_selected_color_presentation();
    assert_eq!(app.editor.buffer_text(1).as_deref(), Some("#112233\n"));
    assert!(app.color_picker.is_none());
}
