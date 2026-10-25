use std::fs;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_editor::Action;

use crate::app::undo_test_support::{app_with, branch, configured_app, edit, save, text};

#[test]
fn g_minus_reaches_the_abandoned_branch() {
    let (dir, mut app) = app_with("tree_gminus", "base\n");
    let old_branch = branch(&mut app);
    let current = text(&app);
    assert_ne!(old_branch, current);
    app.handle_undo_older(false).expect("older");
    assert_eq!(text(&app), old_branch);
    app.handle_undo_older(true).expect("newer");
    assert_eq!(text(&app), current);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn g_minus_key_reaches_the_abandoned_branch() {
    let (dir, mut app) = app_with("tree_gminus_key", "base\n");
    let old_branch = branch(&mut app);
    let current = text(&app);
    let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
    app.handle_key(key(KeyCode::Char('g'))).expect("g");
    app.handle_key(key(KeyCode::Char('-'))).expect("minus");
    assert_eq!(text(&app), old_branch);
    app.handle_key(key(KeyCode::Char('g'))).expect("g");
    app.handle_key(key(KeyCode::Char('+'))).expect("plus");
    assert_eq!(text(&app), current);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn g_minus_round_trips_many_branches() {
    let (dir, mut app) = app_with("tree_roundtrip", "base\n");
    edit(&mut app, "a");
    edit(&mut app, "b");
    app.editor.apply(Action::Undo);
    edit(&mut app, "c");
    let current = text(&app);
    for _ in 0..3 {
        app.handle_undo_older(false).expect("older");
        app.handle_undo_older(true).expect("newer");
        assert_eq!(text(&app), current);
    }
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_multi_step_jump_touches_the_buffer_once() {
    let (dir, mut app) = app_with("tree_revision", "base\n");
    branch(&mut app);
    let revision = app.editor.buffer_revision();
    app.handle_undo_older(false).expect("older");
    assert_eq!(app.editor.buffer_revision(), revision + 1);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn jump_cursor_lands_in_the_changed_region() {
    let (dir, mut app) = app_with("tree_cursor", "one\ntwo\nthree\n");
    app.editor.apply(Action::JumpTo(1, 0));
    edit(&mut app, "a");
    edit(&mut app, "b");
    app.editor.apply(Action::Undo);
    app.editor.apply(Action::JumpTo(1, 0));
    edit(&mut app, "X");
    app.handle_undo_older(false).expect("older");
    assert_eq!(app.editor.row(), 1);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn restart_keeps_the_current_node_and_last_visited_child() {
    let (dir, mut app) = app_with("tree_restart", "base\n");
    let path = dir.join("a.txt");
    app.undo.configure(dir.join("undo"));
    app.undo.set_exclusions(Vec::new());
    app.sync_undo_histories();
    let old_branch = branch(&mut app);
    app.handle_undo_older(false).expect("older");
    assert_eq!(text(&app), old_branch);
    app.editor.apply(Action::Undo);
    save(&mut app);
    drop(app);
    let mut restarted = configured_app(&path, &dir.join("undo"));
    assert!(!restarted.editor.is_dirty());
    assert!(restarted.editor.try_redo());
    assert_eq!(text(&restarted), old_branch);
    let _ = fs::remove_dir_all(dir);
}
