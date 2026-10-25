use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use kawi_buffer::{content_hash, Buffer};
use kawi_editor::Action;
use kawi_lsp::{DocumentEdit, FormattingEvent, TextChange, WorkspaceEdit};

use super::super::save::{PendingSave, SaveAfter};
use super::*;
use crate::keymap::Flavor;

fn app_with(name: &str, text: &str) -> (PathBuf, PathBuf, App) {
    let dir = crate::test_support::temp_dir(name);
    let path = dir.join("a.txt");
    fs::write(&path, text).expect("write");
    let app = configured_app(&path, &dir.join("undo"), Vec::new());
    (dir, path, app)
}

fn configured_app(path: &Path, store: &Path, exclusions: Vec<String>) -> App {
    let mut app = App::new(
        Buffer::from_file(path.to_str().expect("path")).expect("buffer"),
        Flavor::Vim,
    );
    app.undo.configure(store.to_path_buf());
    app.undo.set_exclusions(exclusions);
    app.sync_undo_histories();
    app
}

fn edit(app: &mut App, text: &str) {
    app.editor.apply(Action::EnterInsert);
    for ch in text.chars() {
        app.editor.apply(Action::InsertChar(ch));
    }
    app.editor.apply(Action::ExitInsert);
}

fn save(app: &mut App) {
    app.request_save(false, SaveAfter::None, false);
    assert!(app.pending_save.is_none());
}

fn buffer_text(app: &App) -> String {
    app.editor.buffer_text(1).expect("buffer")
}

fn undo_file_count(dir: &Path) -> usize {
    fs::read_dir(dir)
        .map(|entries| entries.count())
        .unwrap_or_default()
}

#[test]
fn restores_history_after_restart() {
    let (dir, path, mut app) = app_with("restart", "alpha\n");
    edit(&mut app, "x");
    save(&mut app);
    assert_eq!(buffer_text(&app), "xalpha\n");
    drop(app);
    let mut restarted = configured_app(&path, &dir.join("undo"), Vec::new());
    assert!(restarted.editor.try_undo());
    assert_eq!(buffer_text(&restarted), "alpha\n");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn write_as_persists_the_adopted_history() {
    let dir = crate::test_support::temp_dir("write_as");
    let path = dir.join("named.txt");
    let mut app = App::new(Buffer::from_string("alpha\n"), Flavor::Vim);
    app.undo.configure(dir.join("undo"));
    app.undo.set_exclusions(Vec::new());
    edit(&mut app, "x");
    app.run_command(kawi_editor::parse(&format!("w {}", path.display())))
        .expect("write as");
    assert_eq!(fs::read_to_string(&path).expect("disk"), "xalpha\n");
    let mut restarted = configured_app(&path, &dir.join("undo"), Vec::new());
    assert!(restarted.editor.try_undo());
    assert_eq!(buffer_text(&restarted), "alpha\n");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn external_rewrite_discards_history() {
    let (dir, path, mut app) = app_with("rewrite", "alpha\n");
    edit(&mut app, "x");
    save(&mut app);
    fs::write(&path, "different\n").expect("rewrite");
    let mut restarted = configured_app(&path, &dir.join("undo"), Vec::new());
    assert!(!restarted.editor.try_undo());
    assert_eq!(buffer_text(&restarted), "different\n");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn identical_rewrite_restores_history() {
    let (dir, path, mut app) = app_with("identical", "alpha\n");
    edit(&mut app, "x");
    save(&mut app);
    fs::write(&path, "xalpha\n").expect("rewrite");
    let mut restarted = configured_app(&path, &dir.join("undo"), Vec::new());
    assert!(restarted.editor.try_undo());
    assert_eq!(buffer_text(&restarted), "alpha\n");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn last_writer_keeps_a_valid_history() {
    let (dir, path, mut first) = app_with("last_writer", "alpha\n");
    let mut second = configured_app(&path, &dir.join("undo"), Vec::new());
    edit(&mut first, "x");
    save(&mut first);
    edit(&mut second, "y");
    save(&mut second);
    assert_eq!(fs::read_to_string(&path).expect("disk"), "yalpha\n");
    let mut restarted = configured_app(&path, &dir.join("undo"), Vec::new());
    assert!(restarted.editor.try_undo());
    assert_eq!(buffer_text(&restarted), "alpha\n");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn crlf_format_on_save_restores_history() {
    let (dir, path, mut app) = app_with("crlf", "alpha\r\n");
    edit(&mut app, "x");
    let current = buffer_text(&app);
    app.pending_save = Some(PendingSave {
        request: 7,
        number: 1,
        force: false,
        after: SaveAfter::None,
        visible: false,
        started: Instant::now(),
    });
    app.finish_formatted_save(formatting_event(7, &path, &current, "formatted\r\n"));
    assert_eq!(fs::read(&path).expect("disk"), b"formatted\r\n");
    drop(app);
    let mut restarted = configured_app(&path, &dir.join("undo"), Vec::new());
    assert!(restarted.editor.try_undo());
    assert_eq!(buffer_text(&restarted), "xalpha\r\n");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn bom_files_round_trip() {
    let dir = crate::test_support::temp_dir("bom");
    let path = dir.join("a.txt");
    fs::write(&path, b"\xef\xbb\xbfalpha\n").expect("write");
    let mut app = configured_app(&path, &dir.join("undo"), Vec::new());
    edit(&mut app, "x");
    save(&mut app);
    drop(app);
    let mut restarted = configured_app(&path, &dir.join("undo"), Vec::new());
    assert!(restarted.editor.try_undo());
    assert_eq!(buffer_text(&restarted), "\u{feff}alpha\n");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn corrupt_undo_files_open_normally() {
    let (dir, path, mut app) = app_with("corrupt", "alpha\n");
    edit(&mut app, "x");
    save(&mut app);
    let store = dir.join("undo");
    let undo_path = fs::read_dir(&store)
        .expect("store")
        .next()
        .expect("undo file")
        .expect("entry")
        .path();
    fs::write(&undo_path, b"corrupt").expect("corrupt");
    let mut restarted = configured_app(&path, &store, Vec::new());
    assert_eq!(buffer_text(&restarted), "xalpha\n");
    assert!(!restarted.editor.try_undo());
    assert!(!undo_path.exists());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn disabling_persistence_skips_writes() {
    let (dir, _path, mut app) = app_with("disabled", "alpha\n");
    app.undo.set_enabled(false);
    edit(&mut app, "x");
    save(&mut app);
    assert_eq!(undo_file_count(&dir.join("undo")), 0);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn exclusions_skip_persistence() {
    let (dir, _path, mut app) = app_with("excluded", "alpha\n");
    app.undo.set_exclusions(vec!["/tmp/*".to_string()]);
    edit(&mut app, "x");
    save(&mut app);
    assert_eq!(undo_file_count(&dir.join("undo")), 0);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn deleted_files_remove_persisted_history() {
    let (dir, path, mut app) = app_with("deleted", "alpha\n");
    edit(&mut app, "x");
    save(&mut app);
    assert_eq!(undo_file_count(&dir.join("undo")), 1);
    fs::remove_file(&path).expect("delete");
    app.persist_clean_undos();
    assert_eq!(undo_file_count(&dir.join("undo")), 0);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn oversized_files_skip_persistence() {
    let dir = crate::test_support::temp_dir("oversized");
    let path = dir.join("a.txt");
    let text = "x".repeat(MAX_SOURCE_BYTES as usize + 1);
    fs::write(&path, &text).expect("write");
    let mut app = configured_app(&path, &dir.join("undo"), Vec::new());
    edit(&mut app, "y");
    save(&mut app);
    assert_eq!(undo_file_count(&dir.join("undo")), 0);
    let _ = fs::remove_dir_all(dir);
}

fn formatting_event(request: u64, path: &Path, original: &str, formatted: &str) -> FormattingEvent {
    FormattingEvent {
        request,
        edit: WorkspaceEdit {
            documents: vec![DocumentEdit {
                path: path.to_path_buf(),
                original_hash: content_hash(original.as_bytes()),
                edits: vec![TextChange {
                    start: 0,
                    end: original.len(),
                    text: formatted.to_string(),
                }],
            }],
        },
    }
}
