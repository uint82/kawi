use std::path::PathBuf;
use std::sync::mpsc;

use super::*;
use crate::app::directory::planner::Plan;
use crate::app::directory::test_support::FakeFs;
use crate::recovery::{RecoveryManager, RecoveryStart, WorkspaceStore};
use crate::test_support::temp_dir;

fn manager(root: &std::path::Path) -> RecoveryManager {
    let (wake, _wake) = mpsc::channel();
    match RecoveryManager::start(root, root, wake) {
        RecoveryStart::Active { manager, .. } => *manager,
        RecoveryStart::Disabled(warning) => panic!("recovery disabled: {warning}"),
    }
}

fn prepared(plan: &Plan) -> PreparedPlan<'_> {
    let fs = FakeFs::new(&[("/project", true), ("/project/a.txt", false)]);
    PreparedPlan::new(plan, &fs).unwrap()
}

fn plan() -> Plan {
    Plan {
        operations: vec![Operation::Trash(PathBuf::from("/project/a.txt"))],
        changes: 1,
    }
}

#[test]
fn journals_are_persisted_committed_and_removed() {
    let root = temp_dir("directory_journal_lifecycle");
    let manager = manager(&root);
    let plan = plan();
    let prepared = prepared(&plan);
    let journal = create(&prepared).unwrap();
    assert!(matches!(
        journal.operations.first(),
        Some(JournalOperation::Delete { .. })
    ));
    assert!(persist(Some(&manager), &journal).unwrap());
    let store = WorkspaceStore::create(&root, &root).unwrap();
    assert_eq!(
        store.load_directory_journal().unwrap(),
        Some(journal.clone())
    );
    let outcome = Outcome {
        completed: 1,
        remaining: Vec::new(),
        rollback_errors: Vec::new(),
        deferred_deletes: Vec::new(),
        error: None,
    };
    assert!(finish_success(Some(&manager), Some(&journal), &outcome, true).is_none());
    assert_eq!(store.load_directory_journal().unwrap(), None);
    drop(manager);
    std::fs::remove_dir_all(root).unwrap();
}

fn success_with_deferred(path: PathBuf) -> Outcome {
    Outcome {
        completed: 1,
        remaining: Vec::new(),
        rollback_errors: Vec::new(),
        deferred_deletes: vec![path],
        error: None,
    }
}

#[test]
fn successful_apply_removes_deferred_sources_without_recovery() {
    let root = temp_dir("directory_journal_deferred_without_manager");
    let source = root.join(".a.kawi-copy");
    std::fs::write(&source, "alpha").unwrap();
    let outcome = success_with_deferred(source.clone());
    assert!(finish_success(None, None, &outcome, false).is_none());
    assert!(!source.exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn successful_apply_commits_and_removes_deferred_sources() {
    let root = temp_dir("directory_journal_deferred");
    let manager = manager(&root);
    let source = root.join(".a.kawi-copy");
    std::fs::write(&source, "alpha").unwrap();
    let plan = plan();
    let prepared = prepared(&plan);
    let journal = create(&prepared).unwrap();
    assert!(persist(Some(&manager), &journal).unwrap());
    manager.store_clone().append_directory_promotion(0).unwrap();
    let outcome = success_with_deferred(source.clone());
    assert!(finish_success(Some(&manager), Some(&journal), &outcome, true).is_none());
    assert!(!source.exists());
    let store = WorkspaceStore::create(&root, &root).unwrap();
    assert_eq!(store.load_directory_journal().unwrap(), None);
    assert!(store.load_directory_promotions().unwrap().is_empty());
    drop(manager);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn completed_rollbacks_remove_the_journal() {
    let root = temp_dir("directory_journal_completed_rollback");
    let manager = manager(&root);
    let plan = plan();
    let prepared = prepared(&plan);
    let journal = create(&prepared).unwrap();
    assert!(persist(Some(&manager), &journal).unwrap());
    let outcome = Outcome {
        completed: 1,
        remaining: Vec::new(),
        rollback_errors: Vec::new(),
        deferred_deletes: Vec::new(),
        error: Some("failed".to_string()),
    };
    assert!(finish_failure(Some(&manager), &outcome, true).is_none());
    let store = WorkspaceStore::create(&root, &root).unwrap();
    assert_eq!(store.load_directory_journal().unwrap(), None);
    drop(manager);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_rollback_keeps_the_journal_for_the_next_start() {
    let root = temp_dir("directory_journal_failed_rollback");
    let manager = manager(&root);
    let plan = plan();
    let prepared = prepared(&plan);
    let journal = create(&prepared).unwrap();
    assert!(persist(Some(&manager), &journal).unwrap());
    let outcome = Outcome {
        completed: 1,
        remaining: Vec::new(),
        rollback_errors: vec!["cannot restore".to_string()],
        deferred_deletes: Vec::new(),
        error: Some("failed".to_string()),
    };
    let warning = finish_failure(Some(&manager), &outcome, true).expect("journal warning");
    assert!(warning.contains("kept"));
    let store = WorkspaceStore::create(&root, &root).unwrap();
    assert_eq!(store.load_directory_journal().unwrap(), Some(journal));
    drop(manager);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn journals_record_copy_stages() {
    let plan = Plan {
        operations: vec![Operation::Copy {
            from: PathBuf::from("/project/a.txt"),
            to: PathBuf::from("/mnt/b.txt"),
        }],
        changes: 1,
    };
    let fs = FakeFs::new(&[
        ("/project", true),
        ("/project/a.txt", false),
        ("/mnt", true),
    ]);
    let prepared = PreparedPlan::new(&plan, &fs).unwrap();
    let stage = prepared.stage_path(0).unwrap().to_path_buf();
    let journal = create(&prepared).unwrap();
    let JournalOperation::Copy {
        stage: Some(recorded),
        ..
    } = &journal.operations[0]
    else {
        panic!("copy stage was not journaled");
    };
    assert_eq!(recorded.as_path(), stage);
}
