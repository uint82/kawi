use std::fs;
use std::path::PathBuf;

use kawi_editor::Action;

use super::{listing, set_listing, settle, setup};

#[test]
fn watch_paths_refresh_clean_directory_buffers() {
    let (dir, mut app) = setup("directory_watch_refresh", &["alpha.txt"], &[]);
    let beta = dir.join("beta.txt");
    fs::write(&beta, "new\n").unwrap();
    assert!(app.handle_directory_watch_paths(&[beta]));
    settle(&mut app);
    assert_eq!(listing(&app), ["alpha.txt", "beta.txt"]);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn watch_refresh_keeps_the_selected_entry() {
    let (dir, mut app) = setup(
        "directory_watch_selection",
        &["alpha.txt", "beta.txt", "gamma.txt"],
        &[],
    );
    app.editor.apply(Action::JumpTo(1, 0));
    let alpha = dir.join("alpha.txt");
    fs::remove_file(&alpha).unwrap();
    assert!(app.handle_directory_watch_paths(&[alpha]));
    settle(&mut app);
    assert_eq!(listing(&app), ["beta.txt", "gamma.txt"]);
    assert_eq!(app.editor.row(), 0);
    assert_eq!(listing(&app)[app.editor.row()], "beta.txt");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn watch_paths_leave_dirty_directory_buffers_untouched() {
    let (dir, mut app) = setup("directory_watch_dirty", &["alpha.txt"], &[]);
    set_listing(&mut app, "renamed.txt");
    let beta = dir.join("beta.txt");
    fs::write(&beta, "new\n").unwrap();
    assert!(!app.handle_directory_watch_paths(&[beta]));
    settle(&mut app);
    assert_eq!(listing(&app), ["renamed.txt"]);
    assert!(app.editor.is_dirty());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn watch_reloads_without_changes_keep_the_revision() {
    let (dir, mut app) = setup("directory_watch_unchanged", &["alpha.txt"], &[]);
    let number = app.editor.buffer_number();
    let revision = app.editor.buffer_revision_at(number).unwrap();
    let alpha = dir.join("alpha.txt");
    assert!(app.handle_directory_watch_paths(&[alpha]));
    settle(&mut app);
    assert_eq!(app.editor.buffer_revision_at(number), Some(revision));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn unrelated_watch_paths_do_not_start_loads() {
    let (dir, mut app) = setup("directory_watch_unrelated", &["alpha.txt"], &[]);
    assert!(!app.handle_directory_watch_paths(&[PathBuf::from("/outside/entry")]));
    assert!(!app.directory_loading.has_loads());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn deleting_a_directory_buffer_forgets_its_state() {
    let (dir, mut app) = setup("directory_watch_delete", &["alpha.txt"], &[]);
    let number = app.editor.buffer_number();
    app.handle_delete_buffer(Some(number.to_string()), true)
        .unwrap();
    assert!(!app.directory.contains(number));
    fs::remove_dir_all(dir).unwrap();
}
