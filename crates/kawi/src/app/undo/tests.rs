use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use kawi_buffer::{content_hash, Buffer};
use kawi_editor::Action;
use kawi_lsp::{DocumentEdit, FormattingEvent, TextChange, WorkspaceEdit};
use sha2::{Digest, Sha256};

use super::super::save::{PendingSave, SaveAfter, SaveFormat};
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

fn sole_undo_file(dir: &Path) -> PathBuf {
    fs::read_dir(dir)
        .expect("store")
        .next()
        .expect("undo file")
        .expect("entry")
        .path()
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
fn unchanged_histories_are_not_rewritten() {
    let (dir, _path, mut app) = app_with("unchanged_history", "alpha\n");
    edit(&mut app, "x");
    save(&mut app);
    let store = dir.join("undo");
    let undo_file = sole_undo_file(&store);
    fs::remove_file(&undo_file).expect("remove");
    app.persist_clean_undos();
    assert!(!undo_file.exists());
    edit(&mut app, "y");
    save(&mut app);
    assert!(undo_file.exists());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn restored_histories_are_not_rewritten() {
    let (dir, path, mut app) = app_with("restored_history", "alpha\n");
    edit(&mut app, "x");
    save(&mut app);
    drop(app);
    let store = dir.join("undo");
    let undo_file = sole_undo_file(&store);
    let mut restarted = configured_app(&path, &store, Vec::new());
    fs::remove_file(&undo_file).expect("remove");
    restarted.persist_clean_undos();
    assert!(!undo_file.exists());
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
    first.persist_clean_undos();
    second.persist_clean_undos();
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
        format: SaveFormat::Lsp(7),
        number: 1,
        force: false,
        after: SaveAfter::None,
        visible: false,
        started: Instant::now(),
    });
    app.finish_formatted_save(formatting_event(7, &path, &current, "formatted\r\n"));
    let disk = fs::read(&path).expect("disk");
    assert_eq!(disk, b"formatted\r\n");
    assert_eq!(
        app.editor.buffer_source_hash(1),
        Some(Sha256::digest(&disk).into())
    );
    drop(app);
    let mut restarted = configured_app(&path, &dir.join("undo"), Vec::new());
    assert!(restarted.editor.try_undo());
    assert_eq!(buffer_text(&restarted), "xalpha\n");
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
fn undoable_reload_keeps_history() {
    let (dir, path, mut app) = app_with("reload_undoable", "alpha\n");
    edit(&mut app, "x");
    save(&mut app);
    fs::write(&path, "changed\n").expect("rewrite");
    app.editor.reload_buffer(1).expect("reload");
    assert_eq!(buffer_text(&app), "changed\n");
    app.persist_clean_undos();
    let mut restarted = configured_app(&path, &dir.join("undo"), Vec::new());
    assert!(restarted.editor.try_undo());
    assert_eq!(buffer_text(&restarted), "xalpha\n");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn reload_outside_history_drops_the_entry() {
    let (dir, path, mut app) = app_with("reload_discard", "alpha\n");
    edit(&mut app, "x");
    save(&mut app);
    fs::write(&path, "changed\n").expect("rewrite");
    app.editor.discard_current_changes();
    app.persist_clean_undos();
    assert_eq!(undo_file_count(&dir.join("undo")), 0);
    let mut restarted = configured_app(&path, &dir.join("undo"), Vec::new());
    assert!(!restarted.editor.try_undo());
    assert_eq!(buffer_text(&restarted), "changed\n");
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
fn readonly_historical_buffers_are_skipped() {
    let (dir, _path, mut app) = app_with("readonly_history", "alpha\n");
    app.open_readonly_window("HEAD:src/main.rs", "diff\n")
        .expect("historical buffer");
    app.persist_clean_undos();
    assert_eq!(undo_file_count(&dir.join("undo")), 0);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn commit_message_buffers_are_skipped() {
    let dir = crate::test_support::temp_dir("commit_message");
    fs::create_dir_all(dir.join(".git")).expect("git dir");
    let path = dir.join(".git/COMMIT_EDITMSG");
    fs::write(&path, "message\n").expect("write");
    let mut app = configured_app(&path, &dir.join("undo"), Vec::new());
    edit(&mut app, "x");
    save(&mut app);
    assert_eq!(fs::read_to_string(&path).expect("disk"), "xmessage\n");
    assert_eq!(undo_file_count(&dir.join("undo")), 0);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn never_saved_buffers_are_skipped() {
    let dir = crate::test_support::temp_dir("never_saved");
    let mut app = App::new(Buffer::from_string("alpha\n"), Flavor::Vim);
    app.undo.configure(dir.join("undo"));
    app.undo.set_exclusions(Vec::new());
    app.persist_clean_undos();
    assert_eq!(undo_file_count(&dir.join("undo")), 0);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn scratch_buffers_are_skipped() {
    let (dir, _path, mut app) = app_with("scratch", "alpha\n");
    app.editor
        .open_scratch_buffer("scratch.txt", "scratch\n", true);
    app.persist_clean_undos();
    assert_eq!(undo_file_count(&dir.join("undo")), 0);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn failed_undo_writes_do_not_fail_saves() {
    let (dir, path, mut app) = app_with("store_failure", "alpha\n");
    let bad_store = dir.join("undo-store");
    fs::write(&bad_store, "not a directory").expect("block store");
    app.undo.configure(bad_store.clone());
    edit(&mut app, "x");
    save(&mut app);
    assert_eq!(fs::read_to_string(&path).expect("disk"), "xalpha\n");
    assert!(bad_store.is_file());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn exclusions_skip_persistence() {
    let (dir, _path, mut app) = app_with("excluded", "alpha\n");
    let canonical = fs::canonicalize(&dir).unwrap();
    app.undo
        .set_exclusions(vec![format!("{}/*", canonical.display())]);
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
