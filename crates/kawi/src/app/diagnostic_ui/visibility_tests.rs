use super::*;
use crate::app::lsp::DiagnosticUpdate;
use crate::test_support::{diagnostic, diagnostic_event};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;
use kawi_lsp::DiagnosticSeverity;
use std::path::{Path, PathBuf};

fn app_with_error(path: &Path) -> App {
    let mut app = App::new(Buffer::from_string("value\n"), crate::keymap::Flavor::Vim);
    let diagnostics = vec![diagnostic(0, DiagnosticSeverity::Error, "message")];
    app.problems
        .replace_lsp_test("fake", path, diagnostics.clone());
    app.lsp
        .apply_diagnostics_for_test(diagnostic_event("fake", path, diagnostics));
    app
}

fn request(path: &Path) -> VirtualTextRequest<'_> {
    VirtualTextRequest {
        path: Some(path),
        rows: &[0],
        cursor_row: 0,
        cursor_column: 0,
    }
}

fn press(app: &mut App, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
        .expect("key");
}

fn has_underlines(app: &App, lines: &[String], rows: &[usize]) -> bool {
    !app.visible_diagnostic_spans(lines, rows).is_empty()
}

fn has_virtual_text(app: &App, path: &Path) -> bool {
    app.virtual_texts(request(path))[0].is_some()
}

#[test]
fn insert_mode_hides_existing_diagnostic_decorations() {
    let path = PathBuf::from("/tmp/visibility_existing.rs");
    let mut app = app_with_error(&path);
    let lines = vec!["value".to_string()];
    let rows = [0];
    assert!(has_underlines(&app, &lines, &rows));
    assert!(has_virtual_text(&app, &path));
    press(&mut app, KeyCode::Char('i'));
    assert!(!has_underlines(&app, &lines, &rows));
    assert!(!has_virtual_text(&app, &path));
}

#[test]
fn normal_mode_restores_diagnostic_decorations() {
    let path = PathBuf::from("/tmp/visibility_restore.rs");
    let mut app = app_with_error(&path);
    let lines = vec!["value".to_string()];
    let rows = [0];
    press(&mut app, KeyCode::Char('i'));
    press(&mut app, KeyCode::Esc);
    assert!(has_underlines(&app, &lines, &rows));
    assert!(has_virtual_text(&app, &path));
}

#[test]
fn replace_mode_hides_diagnostic_decorations() {
    let path = PathBuf::from("/tmp/visibility_replace.rs");
    let mut app = app_with_error(&path);
    let lines = vec!["value".to_string()];
    let rows = [0];
    press(&mut app, KeyCode::Char('R'));
    assert!(!has_underlines(&app, &lines, &rows));
    assert!(!has_virtual_text(&app, &path));
}

#[test]
fn queued_diagnostic_updates_show_when_insert_mode_ends() {
    let path = PathBuf::from("/tmp/visibility_queued.rs");
    let mut app = App::new(Buffer::from_string("value\n"), crate::keymap::Flavor::Vim);
    press(&mut app, KeyCode::Char('i'));
    let diagnostics = vec![diagnostic(0, DiagnosticSeverity::Error, "message")];
    app.lsp
        .apply_diagnostics_for_test(diagnostic_event("fake", &path, diagnostics.clone()));
    app.lsp
        .queue_diagnostic_update(DiagnosticUpdate::Replace(diagnostic_event(
            "fake",
            &path,
            diagnostics,
        )));
    let lines = vec!["value".to_string()];
    let rows = [0];
    assert!(!has_underlines(&app, &lines, &rows));
    assert!(!has_virtual_text(&app, &path));
    press(&mut app, KeyCode::Esc);
    assert!(app.apply_diagnostic_updates());
    assert!(has_underlines(&app, &lines, &rows));
    assert!(has_virtual_text(&app, &path));
}
