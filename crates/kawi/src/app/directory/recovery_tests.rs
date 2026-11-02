use std::fs;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;

use super::super::save::SaveAfter;
use super::test_support::settle;
use super::tests::{set_listing, setup};
use crate::app::App;
use crate::keymap::Flavor;
use crate::recovery::{DirectoryJournal, JournalOperation, JournalPath, WorkspaceStore};
use crate::test_support::temp_dir;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn path(path: std::path::PathBuf) -> JournalPath {
    JournalPath::from(path)
}

#[test]
fn prepared_directory_recovery_rolls_back_after_confirmation() {
    let root = temp_dir("directory_journal_rollback");
    fs::write(root.join("b.txt"), "renamed").unwrap();
    fs::write(root.join("new.txt"), "new").unwrap();
    fs::write(root.join(".old.kawi-delete"), "old").unwrap();
    let journal = DirectoryJournal::prepared(vec![
        JournalOperation::Delete {
            path: path(root.join("old.txt")),
            staged: path(root.join(".old.kawi-delete")),
        },
        JournalOperation::Rename {
            from: path(root.join("a.txt")),
            to: path(root.join(".a.kawi-move")),
            stage: None,
        },
        JournalOperation::Rename {
            from: path(root.join(".a.kawi-move")),
            to: path(root.join("b.txt")),
            stage: None,
        },
        JournalOperation::CreateFile {
            path: path(root.join("new.txt")),
        },
    ]);
    let store = WorkspaceStore::create(&root, &root).unwrap();
    store.save_directory_journal(&journal).unwrap();
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    assert_eq!(app.start_recovery(&root, &root, false), None);
    let prompt = app.pager_lines().expect("recovery prompt").join("\n");
    assert!(prompt.contains("Interrupted directory operation"));
    app.handle_key(key(KeyCode::Char('r'))).expect("rollback");
    assert!(root.join("a.txt").exists());
    assert!(root.join("old.txt").exists());
    assert!(!root.join("b.txt").exists());
    assert!(!root.join("new.txt").exists());
    assert!(store.load_directory_journal().unwrap().is_none());
    assert!(app
        .last_message()
        .is_some_and(|message| message.contains("Directory recovery")));
    drop(app);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn keeping_directory_recovery_discards_the_journal() {
    let root = temp_dir("directory_journal_keep");
    fs::write(root.join("new.txt"), "new").unwrap();
    let journal = DirectoryJournal::prepared(vec![JournalOperation::CreateFile {
        path: path(root.join("new.txt")),
    }]);
    let store = WorkspaceStore::create(&root, &root).unwrap();
    store.save_directory_journal(&journal).unwrap();
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    assert_eq!(app.start_recovery(&root, &root, false), None);
    app.handle_key(key(KeyCode::Char('k'))).expect("keep");
    assert!(root.join("new.txt").exists());
    assert!(store.load_directory_journal().unwrap().is_none());
    assert!(app
        .last_message()
        .is_some_and(|message| message.contains("discarded")));
    drop(app);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_successful_directory_apply_clears_its_journal() {
    let (project, mut app) = setup("directory_journal_success", &["alpha.txt"], &[]);
    let recovery = temp_dir("directory_journal_success_recovery");
    assert_eq!(app.start_recovery(&recovery, &project, false), None);
    set_listing(&mut app, "beta.txt");
    assert!(!app.request_save(false, SaveAfter::None, true));
    settle(&mut app);
    assert!(project.join("beta.txt").exists());
    let store = WorkspaceStore::create(&recovery, &project).unwrap();
    assert!(store.load_directory_journal().unwrap().is_none());
    drop(app);
    fs::remove_dir_all(project).unwrap();
    fs::remove_dir_all(recovery).unwrap();
}

#[test]
fn committed_directory_recovery_finishes_deferred_removals() {
    let root = temp_dir("directory_journal_committed");
    fs::write(root.join(".a.kawi-move"), "alpha").unwrap();
    fs::write(root.join("b.txt"), "alpha").unwrap();
    let move_path = root.join(".a.kawi-move");
    let journal = DirectoryJournal::prepared(vec![
        JournalOperation::Rename {
            from: path(root.join("a.txt")),
            to: path(move_path.clone()),
            stage: None,
        },
        JournalOperation::Rename {
            from: path(move_path.clone()),
            to: path(root.join("b.txt")),
            stage: Some(path(root.join(".b.txt.kawi-copy-1-0"))),
        },
    ])
    .committed(vec![path(move_path.clone())]);
    let store = WorkspaceStore::create(&root, &root).unwrap();
    store.save_directory_journal(&journal).unwrap();
    store.append_directory_promotion(1).unwrap();
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    assert_eq!(app.start_recovery(&root, &root, false), None);
    assert!(!move_path.exists());
    assert!(root.join("b.txt").exists());
    assert!(store.load_directory_journal().unwrap().is_none());
    assert!(store.load_directory_promotions().unwrap().is_empty());
    assert!(app
        .last_message()
        .is_some_and(|message| message.contains("Directory recovery")));
    drop(app);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn prepared_cross_device_recovery_rolls_back_the_promoted_target() {
    let root = temp_dir("directory_journal_cross_device_rollback");
    fs::write(root.join(".a.kawi-move"), "alpha").unwrap();
    fs::write(root.join("b.txt"), "alpha").unwrap();
    let journal = DirectoryJournal::prepared(vec![
        JournalOperation::Rename {
            from: path(root.join("a.txt")),
            to: path(root.join(".a.kawi-move")),
            stage: None,
        },
        JournalOperation::Rename {
            from: path(root.join(".a.kawi-move")),
            to: path(root.join("b.txt")),
            stage: Some(path(root.join(".b.txt.kawi-copy-1-0"))),
        },
    ]);
    let store = WorkspaceStore::create(&root, &root).unwrap();
    store.save_directory_journal(&journal).unwrap();
    store.append_directory_promotion(1).unwrap();
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    assert_eq!(app.start_recovery(&root, &root, false), None);
    app.handle_key(key(KeyCode::Char('r'))).expect("rollback");
    assert!(root.join("a.txt").exists());
    assert!(!root.join(".a.kawi-move").exists());
    assert!(!root.join("b.txt").exists());
    assert!(store.load_directory_journal().unwrap().is_none());
    assert!(store.load_directory_promotions().unwrap().is_empty());
    drop(app);
    fs::remove_dir_all(root).unwrap();
}
