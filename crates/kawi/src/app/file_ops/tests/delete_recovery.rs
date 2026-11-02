use std::fs;

use crossterm::event::KeyCode;
use kawi_buffer::Buffer;

use super::recovery_support::{journal_path, key, start_app};
use super::*;
use crate::app::App;
use crate::recovery::{ApplyJournal, JournalCleanup, JournalOperation, WorkspaceStore};

#[test]
fn successful_permanent_delete_clears_the_recovery_journal() {
    let dir = temp_dir("file_delete_journal_success");
    let recovery = temp_dir("file_delete_journal_success_recovery");
    let target = dir.join("a.txt");
    fs::write(&target, "alpha\n").unwrap();
    let mut app = App::new(
        Buffer::from_file(target.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    assert_eq!(app.start_recovery(&recovery, &dir, false), None);
    command(&mut app, &format!("delete! {}", target.display()));
    app.handle_key(key(KeyCode::Char('y'))).expect("confirm");
    assert!(!target.exists());
    let store = WorkspaceStore::create(&recovery, &dir).unwrap();
    assert!(store.load_apply_journal().unwrap().is_none());
    drop(app);
    let _ = fs::remove_dir_all(dir);
    let _ = fs::remove_dir_all(recovery);
}

#[test]
fn prepared_file_delete_restores_the_staged_target() {
    let dir = temp_dir("file_delete_journal_rollback");
    let recovery = temp_dir("file_delete_journal_rollback_recovery");
    let target = dir.join("a.txt");
    let staged = dir.join(".a.txt.kawi-delete-0-0");
    fs::write(&staged, "alpha\n").unwrap();
    let journal = ApplyJournal::prepared(vec![JournalOperation::Delete {
        path: journal_path(target.clone()),
        staged: journal_path(staged.clone()),
    }]);
    let store = WorkspaceStore::create(&recovery, &dir).unwrap();
    store.save_apply_journal(&journal).unwrap();
    let mut app = start_app(&dir, &recovery);
    app.handle_key(key(KeyCode::Char('r'))).expect("rollback");
    assert_eq!(fs::read_to_string(&target).unwrap(), "alpha\n");
    assert!(!staged.exists());
    assert!(store.load_apply_journal().unwrap().is_none());
    drop(app);
    let _ = fs::remove_dir_all(dir);
    let _ = fs::remove_dir_all(recovery);
}

#[test]
fn committed_file_delete_removes_the_staged_target() {
    let dir = temp_dir("file_delete_journal_committed");
    let recovery = temp_dir("file_delete_journal_committed_recovery");
    let target = dir.join("a.txt");
    let staged = dir.join(".a.txt.kawi-delete-0-0");
    fs::write(&staged, "alpha\n").unwrap();
    let journal = ApplyJournal::prepared(vec![JournalOperation::Delete {
        path: journal_path(target),
        staged: journal_path(staged.clone()),
    }])
    .committed(vec![JournalCleanup::remove(staged.clone())]);
    let store = WorkspaceStore::create(&recovery, &dir).unwrap();
    store.save_apply_journal(&journal).unwrap();
    let app = start_app(&dir, &recovery);
    assert!(!staged.exists());
    assert!(store.load_apply_journal().unwrap().is_none());
    drop(app);
    let _ = fs::remove_dir_all(dir);
    let _ = fs::remove_dir_all(recovery);
}
