use std::fs;

use kawi_editor::{parse, Action};

use crate::app::undo_test_support::{app_with, branch, edit, save, text};

#[test]
fn undo_sequence_jumps_to_an_absolute_state() {
    let (dir, mut app) = app_with("tree_sequence", "base\n");
    let old_branch = branch(&mut app);
    let old = app
        .editor
        .undo_leaves()
        .into_iter()
        .find(|leaf| !leaf.current)
        .expect("old leaf");
    app.run_command(parse(&format!("undo {}", old.sequence)))
        .expect("undo command");
    assert_eq!(text(&app), old_branch);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn undo_zero_returns_to_the_root_state() {
    let (dir, mut app) = app_with("tree_zero", "base\n");
    branch(&mut app);
    app.run_command(parse("undo 0")).expect("undo zero");
    assert_eq!(text(&app), "base\n");
    assert!(!app.editor.is_dirty());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn earlier_and_later_file_writes_update_dirty_state() {
    let (dir, mut app) = app_with("tree_writes", "base\n");
    edit(&mut app, "a");
    save(&mut app);
    edit(&mut app, "b");
    save(&mut app);
    let second_save = text(&app);
    edit(&mut app, "c");
    save(&mut app);
    let third_save = text(&app);
    app.run_command(parse("earlier 1f")).expect("earlier");
    assert_eq!(text(&app), second_save);
    assert!(app.editor.is_dirty());
    app.run_command(parse("later 1f")).expect("later");
    assert_eq!(text(&app), third_save);
    assert!(!app.editor.is_dirty());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn undolist_shows_leaves_and_save_markers() {
    let (dir, mut app) = app_with("tree_list", "base\n");
    edit(&mut app, "a");
    save(&mut app);
    edit(&mut app, "b");
    save(&mut app);
    app.editor.apply(Action::Undo);
    edit(&mut app, "c");
    app.run_command(parse("undolist")).expect("undolist");
    let pager = app.pager.as_ref().expect("pager");
    assert!(pager[0].contains("number"));
    assert!(pager[0].contains("saved"));
    assert!(pager.iter().any(|line| line.contains('>')));
    assert!(pager.iter().filter(|line| !line.contains("number")).count() >= 2);
    assert!(pager.iter().any(|line| line.contains(" 2")));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn undolist_reports_no_states_for_a_fresh_buffer() {
    let (dir, mut app) = app_with("tree_empty_list", "base\n");
    app.run_command(parse("undolist")).expect("undolist");
    assert!(app.pager.is_none());
    assert_eq!(app.message.as_deref(), Some("No undo states"));
    let _ = fs::remove_dir_all(dir);
}
