use std::fs;
use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;

use super::super::save::SaveAfter;
use crate::app::App;
use crate::keymap::Flavor;
use crate::test_support::temp_dir;

use super::test_support::{message_text, settle};

mod escaping;
mod file_types;
mod metadata;
mod options;
mod permissions;
mod reload;
mod wall;
mod watch;

pub(super) fn setup(name: &str, files: &[&str], directories: &[&str]) -> (PathBuf, App) {
    let dir = temp_dir(name);
    for directory in directories {
        fs::create_dir_all(dir.join(directory)).unwrap();
    }
    for file in files {
        fs::write(dir.join(file), "content\n").unwrap();
    }
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    assert!(app.open_directory_path(&dir, true));
    settle(&mut app);
    (dir, app)
}

pub(super) fn set_listing(app: &mut App, text: &str) {
    let number = app.editor.buffer_number();
    assert!(app.editor.restore_buffer_text(number, text));
    app.sync_current_directory();
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

pub(super) fn listing(app: &App) -> Vec<String> {
    app.editor
        .text_lines()
        .into_iter()
        .filter(|line| !line.is_empty())
        .collect()
}

#[test]
fn opening_a_directory_creates_a_named_buffer_and_skips_watching_it() {
    let (dir, app) = setup("directory_open", &["beta.txt", "alpha.txt"], &["src"]);
    let number = app.editor.buffer_number();
    assert!(app.editor.current_buffer_is_directory());
    assert_eq!(app.editor.filename(), dir.to_string_lossy().as_ref());
    assert_eq!(listing(&app), ["src/", "alpha.txt", "beta.txt"]);
    assert!(app.open_files().iter().all(|(buffer, _)| *buffer != number));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_single_rename_applies_without_confirmation() {
    let (dir, mut app) = setup("directory_rename", &["alpha.txt"], &[]);
    set_listing(&mut app, "beta.txt");
    assert!(!app.request_save(false, SaveAfter::None, true));
    settle(&mut app);
    assert!(!dir.join("alpha.txt").exists());
    assert!(dir.join("beta.txt").exists());
    assert!(!app.editor.is_dirty());
    assert!(!app.directory.has_pending());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn multi_action_plans_wait_for_confirmation() {
    let (dir, mut app) = setup("directory_confirm", &["alpha.txt"], &[]);
    set_listing(&mut app, "beta.txt\nnew/\nsub/new.txt");
    assert!(!app.request_save(false, SaveAfter::None, true));
    settle(&mut app);
    assert!(app.directory.has_pending());
    assert!(dir.join("alpha.txt").exists());
    app.apply_pending_directory(true);
    settle(&mut app);
    assert!(!dir.join("alpha.txt").exists());
    assert!(dir.join("beta.txt").exists());
    assert!(dir.join("new").is_dir());
    assert_eq!(fs::read_to_string(dir.join("sub/new.txt")).unwrap(), "");
    assert_eq!(listing(&app), ["new/", "sub/", "beta.txt"]);
    assert!(!app.editor.is_dirty());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn cancelling_a_plan_leaves_the_filesystem_unchanged() {
    let (dir, mut app) = setup("directory_cancel", &["alpha.txt", "old.txt"], &[]);
    set_listing(&mut app, "beta.txt");
    assert!(!app.request_save(false, SaveAfter::None, true));
    settle(&mut app);
    assert!(app.directory.has_pending());
    app.apply_pending_directory(false);
    assert!(dir.join("alpha.txt").exists());
    assert!(dir.join("old.txt").exists());
    assert!(!app.directory.has_pending());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn files_move_into_existing_subdirectories() {
    let (dir, mut app) = setup("directory_move", &["alpha.txt"], &["sub"]);
    set_listing(&mut app, "sub/\nsub/beta.txt");
    assert!(!app.request_save(false, SaveAfter::None, true));
    settle(&mut app);
    assert!(!dir.join("alpha.txt").exists());
    assert!(dir.join("sub/beta.txt").exists());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn enter_opens_the_selected_file() {
    let (dir, mut app) = setup("directory_enter", &["alpha.txt"], &[]);
    app.editor.apply(kawi_editor::Action::JumpTo(0, 0));
    assert!(app.handle_directory_key(key(KeyCode::Enter)).unwrap());
    assert_eq!(
        app.editor.filename(),
        dir.join("alpha.txt").to_string_lossy().as_ref()
    );
    assert!(!app.editor.current_buffer_is_directory());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn minus_opens_the_parent_and_restores_the_cursor() {
    let (dir, mut app) = setup("directory_parent", &[], &["child"]);
    let child = dir.join("child");
    assert!(app.open_directory_path(&child, true));
    settle(&mut app);
    assert!(app.handle_directory_key(key(KeyCode::Char('-'))).unwrap());
    settle(&mut app);
    assert_eq!(app.editor.filename(), dir.to_string_lossy().as_ref());
    assert_eq!(listing(&app)[app.editor.row()], "child/");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn parent_action_opens_parent_from_a_file() {
    let (dir, mut app) = setup("directory_file_parent_action", &["alpha.txt"], &[]);
    app.editor
        .open_file_buffer(dir.join("alpha.txt").to_string_lossy().as_ref())
        .unwrap();
    assert!(!app
        .dispatch_action(kawi_editor::Action::OpenParentDirectory)
        .unwrap());
    settle(&mut app);
    assert_eq!(app.editor.filename(), dir.to_string_lossy().as_ref());
    assert_eq!(listing(&app)[app.editor.row()], "alpha.txt");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn hidden_entries_can_be_toggled_when_the_buffer_is_clean() {
    let (dir, mut app) = setup("directory_hidden", &[".secret", "alpha.txt"], &[]);
    assert_eq!(listing(&app), ["alpha.txt"]);
    assert!(app.handle_directory_key(key(KeyCode::Char('.'))).unwrap());
    settle(&mut app);
    assert_eq!(listing(&app), [".secret", "alpha.txt"]);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn edited_directory_buffers_count_as_unsaved() {
    let (dir, mut app) = setup("directory_dirty", &["alpha.txt"], &[]);
    set_listing(&mut app, "beta.txt");
    assert!(app.editor.is_dirty());
    assert!(app.editor.has_unsaved());
    assert!(Path::new(&dir.join("alpha.txt")).exists());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn edit_command_opens_a_directory_buffer() {
    let dir = temp_dir("directory_edit_command");
    fs::write(dir.join("alpha.txt"), "content\n").unwrap();
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    app.handle_edit(Some(dir.to_string_lossy().into_owned()), false)
        .unwrap();
    settle(&mut app);
    assert!(app.editor.current_buffer_is_directory());
    assert_eq!(app.editor.filename(), dir.to_string_lossy().as_ref());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn saving_renames_retargets_open_buffers() {
    let (dir, mut app) = setup("directory_retarget", &["alpha.txt"], &[]);
    let alpha = dir.join("alpha.txt");
    app.editor
        .open_file_buffer(alpha.to_string_lossy().as_ref())
        .unwrap();
    let file_buffer = app.editor.buffer_number();
    assert!(app.open_directory_path(&dir, true));
    settle(&mut app);
    set_listing(&mut app, "beta.txt");
    assert!(!app.request_save(false, SaveAfter::None, true));
    settle(&mut app);
    assert_eq!(
        app.editor.buffer_path(file_buffer),
        Some(dir.join("beta.txt").to_string_lossy().as_ref())
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn sessions_restore_directory_buffers() {
    let (dir, mut app) = setup(
        "directory_session",
        &["alpha.txt", "beta.txt", "gamma.txt"],
        &[],
    );
    app.editor.apply(kawi_editor::Action::JumpTo(2, 0));
    let session = app.session_snapshot().expect("snapshot");
    let mut restored = App::new(Buffer::from_string(""), Flavor::Vim);
    assert_eq!(restored.restore_session(&session), None);
    settle(&mut restored);
    assert!(restored.editor.current_buffer_is_directory());
    assert_eq!(restored.editor.filename(), dir.to_string_lossy().as_ref());
    assert_eq!(listing(&restored), ["alpha.txt", "beta.txt", "gamma.txt"]);
    assert_eq!(restored.editor.row(), 2);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn minus_in_a_file_buffer_is_left_to_vim() {
    let (dir, mut app) = setup("directory_file_parent", &["alpha.txt"], &[]);
    app.editor
        .open_file_buffer(dir.join("alpha.txt").to_string_lossy().as_ref())
        .unwrap();
    assert!(!app.handle_directory_key(key(KeyCode::Char('-'))).unwrap());
    assert_eq!(
        app.editor.filename(),
        dir.join("alpha.txt").to_string_lossy().as_ref()
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn partial_failures_reload_the_listing_from_disk() {
    let (dir, mut app) = setup("directory_partial", &["alpha.txt"], &[]);
    set_listing(&mut app, "beta.txt\nnew.txt");
    assert!(!app.request_save(false, SaveAfter::None, true));
    settle(&mut app);
    fs::write(dir.join("new.txt"), "external\n").unwrap();
    app.apply_pending_directory(true);
    settle(&mut app);
    assert!(dir.join("beta.txt").exists());
    assert!(!dir.join("alpha.txt").exists());
    assert_eq!(listing(&app), ["beta.txt", "new.txt"]);
    assert!(!app.editor.is_dirty());
    let message = message_text(&app);
    assert!(message.contains("applied 2 operation(s)"));
    assert!(message.contains("beta.txt"));
    assert!(message.contains("no rollback attempted"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn permanent_delete_removes_entries_when_trash_is_disabled() {
    let (dir, mut app) = setup("directory_delete", &["old.txt", "keep.txt"], &[]);
    app.directory_options.trash = false;
    app.directory_options.confirm = false;
    set_listing(&mut app, "keep.txt");
    assert!(!app.request_save(false, SaveAfter::None, true));
    settle(&mut app);
    assert!(!dir.join("old.txt").exists());
    assert!(dir.join("keep.txt").exists());
    assert_eq!(listing(&app), ["keep.txt"]);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn write_quit_finishes_after_the_background_check() {
    let (dir, mut app) = setup("directory_write_quit", &["alpha.txt"], &[]);
    set_listing(&mut app, "beta.txt");
    assert!(!app.request_save(false, SaveAfter::Quit, true));
    settle(&mut app);
    assert!(app.quit_pending);
    assert!(!dir.join("alpha.txt").exists());
    assert!(dir.join("beta.txt").exists());
    let _ = fs::remove_dir_all(dir);
}
