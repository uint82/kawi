use std::fs;

use crossterm::event::KeyCode;

use super::recovery_support::{journal_path, key, start_app};
use super::*;
use crate::recovery::{ApplyJournal, JournalOperation, WorkspaceStore};

#[test]
fn successful_rename_clears_the_recovery_journal() {
    let dir = temp_dir("file_rename_journal_success");
    let recovery = temp_dir("file_rename_journal_success_recovery");
    let source = dir.join("a.txt");
    let target = dir.join("b.txt");
    fs::write(&source, "alpha\n").unwrap();
    let mut app = app_for(&source);
    assert_eq!(app.start_recovery(&recovery, &dir, false), None);
    command(&mut app, &format!("rename {}", target.display()));
    assert!(!source.exists());
    assert_eq!(fs::read_to_string(&target).unwrap(), "alpha\n");
    let store = WorkspaceStore::create(&recovery, &dir).unwrap();
    assert!(store.load_apply_journal().unwrap().is_none());
    assert!(store.load_apply_promotions().unwrap().is_empty());
    drop(app);
    let _ = fs::remove_dir_all(dir);
    let _ = fs::remove_dir_all(recovery);
}

#[test]
fn forced_rename_replaces_the_target_and_clears_the_journal() {
    let dir = temp_dir("file_rename_force_journal");
    let recovery = temp_dir("file_rename_force_journal_recovery");
    let source = dir.join("a.txt");
    let target = dir.join("b.txt");
    fs::write(&source, "new\n").unwrap();
    fs::write(&target, "old\n").unwrap();
    let mut app = app_for(&source);
    assert_eq!(app.start_recovery(&recovery, &dir, false), None);
    command(&mut app, &format!("rename! {}", target.display()));
    assert!(!source.exists());
    assert_eq!(fs::read_to_string(&target).unwrap(), "new\n");
    let store = WorkspaceStore::create(&recovery, &dir).unwrap();
    assert!(store.load_apply_journal().unwrap().is_none());
    drop(app);
    let _ = fs::remove_dir_all(dir);
    let _ = fs::remove_dir_all(recovery);
}

#[test]
fn prepared_file_rename_rolls_back_the_target() {
    let dir = temp_dir("file_rename_journal_rollback");
    let recovery = temp_dir("file_rename_journal_rollback_recovery");
    let source = dir.join("a.txt");
    let target = dir.join("b.txt");
    fs::write(&target, "alpha\n").unwrap();
    let journal = ApplyJournal::prepared(vec![JournalOperation::Rename {
        from: journal_path(source.clone()),
        to: journal_path(target.clone()),
        stage: None,
        force: false,
    }]);
    let store = WorkspaceStore::create(&recovery, &dir).unwrap();
    store.save_apply_journal(&journal).unwrap();
    let mut app = start_app(&dir, &recovery);
    app.handle_key(key(KeyCode::Char('r'))).expect("rollback");
    assert_eq!(fs::read_to_string(&source).unwrap(), "alpha\n");
    assert!(!target.exists());
    assert!(store.load_apply_journal().unwrap().is_none());
    drop(app);
    let _ = fs::remove_dir_all(dir);
    let _ = fs::remove_dir_all(recovery);
}

#[cfg(target_os = "linux")]
#[test]
fn cross_device_file_rename_clears_stages_and_journal() {
    use std::os::unix::fs::MetadataExt;

    let shared = Path::new("/dev/shm");
    if !shared.is_dir() {
        return;
    }
    let dir = temp_dir("file_rename_cross_device");
    let target_dir = shared.join(format!("kawi_cross_device_{}", std::process::id()));
    let _ = fs::remove_dir_all(&target_dir);
    fs::create_dir_all(&target_dir).unwrap();
    let source = dir.join("a.txt");
    let target = target_dir.join("b.txt");
    fs::write(&source, "alpha\n").unwrap();
    if fs::metadata(&source).unwrap().dev() == fs::metadata(&target_dir).unwrap().dev() {
        let _ = fs::remove_dir_all(dir);
        let _ = fs::remove_dir_all(target_dir);
        return;
    }
    let recovery = temp_dir("file_rename_cross_device_recovery");
    let mut app = app_for(&source);
    assert_eq!(app.start_recovery(&recovery, &dir, false), None);
    command(&mut app, &format!("rename {}", target.display()));
    assert!(!source.exists());
    assert_eq!(fs::read_to_string(&target).unwrap(), "alpha\n");
    assert!(!dir
        .read_dir()
        .unwrap()
        .flatten()
        .any(|entry| entry.file_name().to_string_lossy().contains(".kawi-copy-")));
    let store = WorkspaceStore::create(&recovery, &dir).unwrap();
    assert!(store.load_apply_journal().unwrap().is_none());
    drop(app);
    let _ = fs::remove_dir_all(dir);
    let _ = fs::remove_dir_all(target_dir);
    let _ = fs::remove_dir_all(recovery);
}
