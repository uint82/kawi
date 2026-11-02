use std::fs;

use kawi_buffer::Buffer;
use kawi_editor::Action;

use super::*;
use crate::keymap::Flavor;

fn temp_dir(name: &str) -> PathBuf {
    crate::test_support::temp_dir(name)
}

fn app_with(dir: &Path, name: &str) -> App {
    let path = dir.join(name);
    let mut app = App::new(
        Buffer::from_file(path.to_str().expect("path")).expect("buffer"),
        Flavor::Vim,
    );
    app.sync_watched_files();
    app
}

fn edit(app: &mut App, text: &str) {
    app.editor.apply(Action::EnterInsert);
    for ch in text.chars() {
        app.editor.apply(Action::InsertChar(ch));
    }
    app.editor.apply(Action::ExitInsert);
}

fn message(app: &App) -> String {
    app.last_message()
        .map(str::to_string)
        .or_else(|| app.pager_lines().map(|lines| lines.join(" ")))
        .unwrap_or_default()
}

#[test]
fn reloads_clean_buffers_and_preserves_the_cursor() {
    let dir = temp_dir("reload");
    let path = dir.join("a.txt");
    fs::write(&path, "one\ntwo\nthree\n").expect("write");
    let mut app = app_with(&dir, "a.txt");
    app.editor.apply(Action::JumpTo(2, 2));
    fs::write(&path, "changed\n").expect("external");
    app.handle_disk_paths(&[path]);
    assert_eq!(app.editor.visible_lines(0, 1), vec!["changed"]);
    assert_eq!(app.cursor_position(), (0, 2));
    assert!(app.disk_status(1).is_none());
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn identical_content_does_not_reload() {
    let dir = temp_dir("same");
    let path = dir.join("a.txt");
    fs::write(&path, "alpha\n").expect("write");
    let mut app = app_with(&dir, "a.txt");
    let revision = app.editor.buffer_revision();
    fs::write(&path, "alpha\n").expect("rewrite");
    app.handle_disk_paths(&[path]);
    assert_eq!(app.editor.buffer_revision(), revision);
    assert!(app.disk_status(1).is_none());
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn dirty_buffers_become_conflicts_without_overwriting() {
    let dir = temp_dir("dirty");
    let path = dir.join("a.txt");
    fs::write(&path, "alpha\n").expect("write");
    let mut app = app_with(&dir, "a.txt");
    edit(&mut app, "x");
    fs::write(&path, "theirs\n").expect("external");
    app.handle_disk_paths(&[path]);
    assert_eq!(app.editor.visible_lines(0, 1), vec!["xalpha"]);
    assert!(app.editor.is_dirty());
    assert_eq!(app.disk_status(1), Some(STATUS_CHANGED));
    assert!(app.disk_conflict(1));
    assert!(message(&app).ends_with("a.txt"));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn keep_mine_accepts_current_disk_state() {
    let dir = temp_dir("keep");
    let path = dir.join("a.txt");
    fs::write(&path, "alpha\n").expect("write");
    let mut app = app_with(&dir, "a.txt");
    edit(&mut app, "x");
    fs::write(&path, "theirs\n").expect("external");
    app.handle_disk_paths(std::slice::from_ref(&path));
    app.keep_buffer_version(1);
    assert!(!app.disk_conflict(1));
    assert!(app.disk_status(1).is_none());
    assert_eq!(app.editor.visible_lines(0, 1), vec!["xalpha"]);
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn load_theirs_replaces_dirty_content() {
    let dir = temp_dir("theirs");
    let path = dir.join("a.txt");
    fs::write(&path, "alpha\n").expect("write");
    let mut app = app_with(&dir, "a.txt");
    edit(&mut app, "x");
    fs::write(&path, "theirs\n").expect("external");
    app.handle_disk_paths(std::slice::from_ref(&path));
    app.load_disk_version(1).expect("load");
    assert_eq!(app.editor.visible_lines(0, 1), vec!["theirs"]);
    assert!(!app.editor.is_dirty());
    assert!(!app.disk_conflict(1));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn deletion_keeps_the_buffer_and_sets_a_flag() {
    let dir = temp_dir("deleted");
    let path = dir.join("a.txt");
    fs::write(&path, "alpha\n").expect("write");
    let mut app = app_with(&dir, "a.txt");
    fs::remove_file(&path).expect("remove");
    app.handle_disk_paths(&[path]);
    assert_eq!(app.editor.visible_lines(0, 1), vec!["alpha"]);
    assert_eq!(app.disk_status(1), Some(STATUS_DELETED));
    assert!(message(&app).contains("Deleted on disk"));
    assert!(message(&app).contains("kept"));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn external_renames_retarget_clean_buffers() {
    let dir = temp_dir("rename_clean");
    let path = dir.join("a.txt");
    let moved = dir.join("b.txt");
    fs::write(&path, "alpha\ntwo\n").expect("write");
    let mut app = app_with(&dir, "a.txt");
    app.editor.apply(Action::JumpTo(1, 1));
    let revision = app.editor.buffer_revision();
    let cursor = app.cursor_position();
    fs::rename(&path, &moved).expect("rename");
    app.handle_disk_paths(std::slice::from_ref(&path));
    assert_eq!(app.editor.filename(), moved.to_string_lossy());
    assert_eq!(app.editor.buffer_revision(), revision);
    assert_eq!(app.cursor_position(), cursor);
    assert_eq!(app.disk_status(1), None);
    assert!(message(&app).starts_with("File moved on disk"));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn external_renames_keep_dirty_buffers_dirty() {
    let dir = temp_dir("rename_dirty");
    let path = dir.join("a.txt");
    let moved = dir.join("b.txt");
    fs::write(&path, "alpha\n").expect("write");
    let mut app = app_with(&dir, "a.txt");
    edit(&mut app, "x");
    fs::rename(&path, &moved).expect("rename");
    app.handle_disk_paths(std::slice::from_ref(&path));
    assert_eq!(app.editor.filename(), moved.to_string_lossy());
    assert_eq!(app.editor.visible_lines(0, 1), vec!["xalpha"]);
    assert!(app.editor.is_dirty());
    assert_eq!(app.disk_status(1), None);
    assert!(!app.disk_conflict(1));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn external_renames_preserve_unresolved_conflicts() {
    let dir = temp_dir("rename_conflict");
    let path = dir.join("a.txt");
    let moved = dir.join("b.txt");
    fs::write(&path, "alpha\n").expect("write");
    let mut app = app_with(&dir, "a.txt");
    edit(&mut app, "x");
    fs::write(&path, "theirs\n").expect("external");
    app.handle_disk_paths(std::slice::from_ref(&path));
    fs::rename(&path, &moved).expect("rename");
    app.handle_disk_paths(std::slice::from_ref(&path));
    assert_eq!(app.editor.filename(), moved.to_string_lossy());
    assert_eq!(app.disk_status(1), Some(STATUS_CHANGED));
    assert!(app.disk_conflict(1));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn external_renames_find_destinations_in_other_open_directories() {
    let first = temp_dir("rename_first");
    let second = temp_dir("rename_second");
    let path = first.join("a.txt");
    let other = second.join("b.txt");
    let moved = second.join("moved.txt");
    fs::write(&path, "alpha\n").expect("write");
    fs::write(&other, "beta\n").expect("write");
    let mut app = app_with(&first, "a.txt");
    app.editor
        .open_file_buffer(other.to_str().expect("path"))
        .expect("open");
    app.sync_watched_files();
    fs::rename(&path, &moved).expect("rename");
    app.handle_disk_paths(std::slice::from_ref(&path));
    assert_eq!(app.editor.buffer_path(1), Some(moved.to_str().unwrap()));
    assert_eq!(app.disk_status(1), None);
    fs::remove_dir_all(first).expect("cleanup");
    fs::remove_dir_all(second).expect("cleanup");
}

#[test]
fn recreated_files_refresh_identity_before_renames() {
    let dir = temp_dir("rename_recreated");
    let path = dir.join("a.txt");
    let moved = dir.join("b.txt");
    fs::write(&path, "alpha\n").expect("write");
    let mut app = app_with(&dir, "a.txt");
    fs::remove_file(&path).expect("remove");
    fs::write(&path, "alpha\n").expect("recreate");
    app.handle_disk_paths(std::slice::from_ref(&path));
    fs::rename(&path, &moved).expect("rename");
    app.handle_disk_paths(std::slice::from_ref(&path));
    assert_eq!(app.editor.filename(), moved.to_string_lossy());
    assert_eq!(app.disk_status(1), None);
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn external_renames_outside_watched_directories_mark_deleted() {
    let dir = temp_dir("rename_outside");
    let path = dir.join("a.txt");
    let destination = temp_dir("rename_destination");
    let moved = destination.join("a.txt");
    fs::write(&path, "alpha\n").expect("write");
    let mut app = app_with(&dir, "a.txt");
    fs::rename(&path, &moved).expect("rename");
    app.handle_disk_paths(std::slice::from_ref(&path));
    assert_eq!(app.editor.filename(), path.to_string_lossy());
    assert_eq!(app.disk_status(1), Some(STATUS_DELETED));
    assert!(message(&app).starts_with("Deleted on disk"));
    fs::remove_dir_all(destination).expect("cleanup");
    fs::remove_dir_all(dir).expect("cleanup");
}

#[cfg(unix)]
#[test]
fn hardlinked_files_are_not_followed() {
    let dir = temp_dir("rename_hardlink");
    let path = dir.join("a.txt");
    let moved = dir.join("b.txt");
    fs::write(&path, "alpha\n").expect("write");
    fs::hard_link(&path, dir.join("link.txt")).expect("hard link");
    let mut app = app_with(&dir, "a.txt");
    fs::rename(&path, &moved).expect("rename");
    app.handle_disk_paths(std::slice::from_ref(&path));
    assert_eq!(app.editor.filename(), path.to_string_lossy());
    assert_eq!(app.disk_status(1), Some(STATUS_DELETED));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn saving_records_the_new_disk_fingerprint() {
    let dir = temp_dir("save");
    let path = dir.join("a.txt");
    fs::write(&path, "alpha\n").expect("write");
    let mut app = app_with(&dir, "a.txt");
    edit(&mut app, "x");
    app.handle_save(false).expect("save");
    app.handle_disk_paths(std::slice::from_ref(&path));
    assert!(app.disk_status(1).is_none());
    assert_eq!(fs::read_to_string(path).expect("read"), "xalpha\n");
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn recreated_identical_files_clear_deleted_flags() {
    let dir = temp_dir("restored");
    let path = dir.join("a.txt");
    fs::write(&path, "alpha\n").expect("write");
    let mut app = app_with(&dir, "a.txt");
    fs::remove_file(&path).expect("remove");
    app.handle_disk_paths(std::slice::from_ref(&path));
    fs::write(&path, "alpha\n").expect("recreate");
    app.handle_disk_paths(&[path]);
    assert!(app.disk_status(1).is_none());
    assert_eq!(app.editor.visible_lines(0, 1), vec!["alpha"]);
    fs::remove_dir_all(dir).expect("cleanup");
}
