use std::fs;

use super::{set_listing, settle, setup};
use crate::test_support::temp_dir;

#[test]
fn wall_saves_all_dirty_directory_buffers() {
    let (first, mut app) = setup("directory_wall_first", &["alpha.txt"], &[]);
    set_listing(&mut app, "beta.txt");
    let second = temp_dir("directory_wall_second");
    fs::write(second.join("gamma.txt"), "content\n").unwrap();
    assert!(app.open_directory_path(&second, true));
    settle(&mut app);
    set_listing(&mut app, "delta.txt");
    assert!(!app.handle_save_all().unwrap());
    settle(&mut app);
    assert!(app.directory.has_pending());
    app.apply_pending_directory(true);
    settle(&mut app);
    assert!(!first.join("alpha.txt").exists());
    assert!(first.join("beta.txt").exists());
    assert!(!second.join("gamma.txt").exists());
    assert!(second.join("delta.txt").exists());
    assert!(!app.editor.has_unsaved());
    let _ = fs::remove_dir_all(first);
    let _ = fs::remove_dir_all(second);
}

#[test]
fn wall_aborts_when_any_directory_changed_on_disk() {
    let (first, mut app) = setup(
        "directory_wall_abort_first",
        &["alpha.txt", "notes.txt"],
        &[],
    );
    set_listing(&mut app, "beta.txt\nnotes.txt");
    let second = temp_dir("directory_wall_abort_second");
    fs::write(second.join("gamma.txt"), "content\n").unwrap();
    assert!(app.open_directory_path(&second, true));
    settle(&mut app);
    set_listing(&mut app, "delta.txt");
    let notes = first.join("notes.txt");
    app.editor
        .open_file_buffer(notes.to_string_lossy().as_ref())
        .unwrap();
    app.editor.apply(kawi_editor::Action::EnterInsert);
    app.editor.apply(kawi_editor::Action::InsertChar('x'));
    app.editor.apply(kawi_editor::Action::ExitInsert);
    fs::write(second.join("external.txt"), "external\n").unwrap();
    assert!(!app.handle_save_all().unwrap());
    settle(&mut app);
    assert!(first.join("alpha.txt").exists());
    assert!(!first.join("beta.txt").exists());
    assert!(second.join("gamma.txt").exists());
    assert!(!second.join("delta.txt").exists());
    assert_eq!(fs::read_to_string(&notes).unwrap(), "content\n");
    assert!(app.editor.has_unsaved());
    assert!(!app
        .editor
        .buffer_is_readonly(app.editor.buffer_number())
        .unwrap());
    let _ = fs::remove_dir_all(first);
    let _ = fs::remove_dir_all(second);
}

#[test]
fn wall_confirms_all_directory_changes_once() {
    let (first, mut app) = setup("directory_wall_confirm_first", &["alpha.txt"], &[]);
    set_listing(&mut app, "beta.txt\nnew.txt");
    let second = temp_dir("directory_wall_confirm_second");
    fs::write(second.join("gamma.txt"), "content\n").unwrap();
    assert!(app.open_directory_path(&second, true));
    settle(&mut app);
    set_listing(&mut app, "delta.txt\nother.txt");
    assert!(!app.handle_save_all().unwrap());
    settle(&mut app);
    assert!(app.directory.has_pending());
    assert!(first.join("alpha.txt").exists());
    assert!(second.join("gamma.txt").exists());
    app.apply_pending_directory(true);
    settle(&mut app);
    assert!(first.join("beta.txt").exists());
    assert!(first.join("new.txt").exists());
    assert!(second.join("delta.txt").exists());
    assert!(second.join("other.txt").exists());
    let _ = fs::remove_dir_all(first);
    let _ = fs::remove_dir_all(second);
}

#[test]
fn wall_cancellation_keeps_file_buffers_unsaved() {
    let (dir, mut app) = setup("directory_wall_cancel", &["alpha.txt", "notes.txt"], &[]);
    let notes = dir.join("notes.txt");
    set_listing(&mut app, "beta.txt\nnotes.txt\nnew.txt");
    app.editor
        .open_file_buffer(notes.to_string_lossy().as_ref())
        .unwrap();
    app.editor.apply(kawi_editor::Action::EnterInsert);
    app.editor.apply(kawi_editor::Action::InsertChar('x'));
    app.editor.apply(kawi_editor::Action::ExitInsert);
    assert!(!app.handle_save_all().unwrap());
    settle(&mut app);
    assert!(app.directory.has_pending());
    app.apply_pending_directory(false);
    assert_eq!(fs::read_to_string(&notes).unwrap(), "content\n");
    assert!(dir.join("alpha.txt").exists());
    assert!(app.editor.has_unsaved());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn wall_rejects_nested_dirty_directories() {
    let (dir, mut app) = setup("directory_wall_nested", &["alpha.txt"], &["child"]);
    let root = app.editor.buffer_number();
    assert!(app.open_directory_path(&dir.join("child"), true));
    settle(&mut app);
    set_listing(&mut app, "new.txt");
    app.editor.buffer_select(root);
    set_listing(&mut app, "child/\nbeta.txt");
    assert!(!app.handle_save_all().unwrap());
    assert_eq!(
        app.last_message(),
        Some("Nested directory buffers must be saved separately")
    );
    assert!(dir.join("alpha.txt").exists());
    assert!(!dir.join("child/new.txt").exists());
    assert!(app.editor.has_unsaved());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn wall_saves_file_buffers_with_directory_changes() {
    let (dir, mut app) = setup("directory_wall_files", &["alpha.txt", "notes.txt"], &[]);
    let notes = dir.join("notes.txt");
    set_listing(&mut app, "beta.txt\nnotes.txt");
    app.editor
        .open_file_buffer(notes.to_string_lossy().as_ref())
        .unwrap();
    app.editor.apply(kawi_editor::Action::EnterInsert);
    app.editor.apply(kawi_editor::Action::InsertChar('x'));
    app.editor.apply(kawi_editor::Action::ExitInsert);
    assert!(!app.handle_save_all().unwrap());
    settle(&mut app);
    assert!(dir.join("beta.txt").exists());
    assert_eq!(fs::read_to_string(&notes).unwrap(), "xcontent\n");
    assert!(!app.editor.has_unsaved());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn write_quit_all_finishes_directory_transaction_then_quits() {
    let (dir, mut app) = setup("directory_wall_quit", &["alpha.txt"], &[]);
    set_listing(&mut app, "beta.txt");
    assert!(!app.handle_write_quit_all().unwrap());
    settle(&mut app);
    assert!(app.quit_pending);
    assert!(dir.join("beta.txt").exists());
    assert!(!dir.join("alpha.txt").exists());
    let _ = fs::remove_dir_all(dir);
}
