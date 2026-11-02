use std::path::PathBuf;

use super::*;
use crate::app::directory::test_support::FakeFs;
use crate::recovery::JournalPath;

fn operation_paths(paths: &[(&str, bool)]) -> FakeFs {
    FakeFs::new(paths)
}

#[test]
fn recovery_reverses_applied_operations_in_reverse_order() {
    let mut fs = operation_paths(&[
        ("/project", true),
        ("/project/b.txt", false),
        ("/project/new.txt", false),
        ("/project/newdir", true),
        ("/project/copied", false),
        ("/project/.old.kawi-delete", false),
    ]);
    let journal = DirectoryJournal::prepared(vec![
        JournalOperation::Delete {
            path: JournalPath::from(PathBuf::from("/project/old.txt")),
            staged: JournalPath::from(PathBuf::from("/project/.old.kawi-delete")),
        },
        JournalOperation::Rename {
            from: JournalPath::from(PathBuf::from("/project/a.txt")),
            to: JournalPath::from(PathBuf::from("/project/.a.kawi-move")),
            stage: None,
        },
        JournalOperation::Rename {
            from: JournalPath::from(PathBuf::from("/project/.a.kawi-move")),
            to: JournalPath::from(PathBuf::from("/project/b.txt")),
            stage: None,
        },
        JournalOperation::CreateDirectory {
            path: JournalPath::from(PathBuf::from("/project/newdir")),
        },
        JournalOperation::CreateFile {
            path: JournalPath::from(PathBuf::from("/project/new.txt")),
        },
        JournalOperation::Copy {
            from: JournalPath::from(PathBuf::from("/project/source")),
            to: JournalPath::from(PathBuf::from("/project/copied")),
            stage: None,
        },
    ]);
    let report = recover(&journal, &HashSet::new(), &mut fs);
    assert_eq!(report.restored, 3);
    assert_eq!(report.removed, 3);
    assert!(report.irreversible.is_empty());
    assert!(report.skipped.is_empty());
    assert!(report.errors.is_empty());
    assert!(fs.exists(std::path::Path::new("/project/a.txt")));
    assert!(fs.exists(std::path::Path::new("/project/old.txt")));
    assert!(!fs.exists(std::path::Path::new("/project/b.txt")));
    assert!(!fs.exists(std::path::Path::new("/project/new.txt")));
    assert!(!fs.exists(std::path::Path::new("/project/newdir")));
    assert!(!fs.exists(std::path::Path::new("/project/copied")));
}

#[test]
fn finalized_deletes_are_reported_as_irreversible() {
    let mut fs = operation_paths(&[("/project", true)]);
    let journal = DirectoryJournal::prepared(vec![JournalOperation::Delete {
        path: JournalPath::from(PathBuf::from("/project/old.txt")),
        staged: JournalPath::from(PathBuf::from("/project/.old.kawi-delete")),
    }]);
    let report = recover(&journal, &HashSet::new(), &mut fs);
    assert_eq!(report.irreversible, vec!["/project/old.txt".to_string()]);
    assert_eq!(report.restored, 0);
    assert_eq!(report.removed, 0);
}

#[test]
fn external_targets_are_left_in_place_when_the_original_still_exists() {
    let mut fs = operation_paths(&[
        ("/project", true),
        ("/project/a.txt", false),
        ("/project/b.txt", false),
    ]);
    let journal = DirectoryJournal::prepared(vec![
        JournalOperation::Rename {
            from: JournalPath::from(PathBuf::from("/project/a.txt")),
            to: JournalPath::from(PathBuf::from("/project/.a.kawi-move")),
            stage: None,
        },
        JournalOperation::Rename {
            from: JournalPath::from(PathBuf::from("/project/.a.kawi-move")),
            to: JournalPath::from(PathBuf::from("/project/b.txt")),
            stage: None,
        },
    ]);
    let report = recover(&journal, &HashSet::new(), &mut fs);
    assert_eq!(report.restored, 0);
    assert_eq!(report.skipped.len(), 1);
    assert!(fs.exists(std::path::Path::new("/project/a.txt")));
    assert!(fs.exists(std::path::Path::new("/project/b.txt")));
}

#[test]
fn ambiguous_paths_are_skipped_instead_of_removed() {
    let mut fs = operation_paths(&[
        ("/project", true),
        ("/project/a.txt", false),
        ("/project/.a.kawi-move", false),
    ]);
    let journal = DirectoryJournal::prepared(vec![JournalOperation::Rename {
        from: JournalPath::from(PathBuf::from("/project/a.txt")),
        to: JournalPath::from(PathBuf::from("/project/.a.kawi-move")),
        stage: None,
    }]);
    let report = recover(&journal, &HashSet::new(), &mut fs);
    assert_eq!(report.skipped.len(), 1);
    assert_eq!(report.restored, 0);
    assert!(fs.exists(std::path::Path::new("/project/a.txt")));
    assert!(fs.exists(std::path::Path::new("/project/.a.kawi-move")));
}

#[test]
fn promoted_cross_device_rename_removes_the_target_and_restores_the_source() {
    let mut fs = operation_paths(&[
        ("/project", true),
        ("/mnt", true),
        ("/project/.a.kawi-move", false),
        ("/mnt/b.txt", false),
    ]);
    let journal = DirectoryJournal::prepared(vec![
        JournalOperation::Rename {
            from: JournalPath::from(PathBuf::from("/project/a.txt")),
            to: JournalPath::from(PathBuf::from("/project/.a.kawi-move")),
            stage: None,
        },
        JournalOperation::Rename {
            from: JournalPath::from(PathBuf::from("/project/.a.kawi-move")),
            to: JournalPath::from(PathBuf::from("/mnt/b.txt")),
            stage: Some(JournalPath::from(PathBuf::from(
                "/mnt/.b.txt.kawi-copy-1-0",
            ))),
        },
    ]);
    let report = recover(&journal, &HashSet::from([1]), &mut fs);
    assert!(report.errors.is_empty());
    assert!(report.skipped.is_empty());
    assert_eq!(report.removed, 1);
    assert_eq!(report.restored, 1);
    assert!(fs.exists(std::path::Path::new("/project/a.txt")));
    assert!(!fs.exists(std::path::Path::new("/project/.a.kawi-move")));
    assert!(!fs.exists(std::path::Path::new("/mnt/b.txt")));
}

#[test]
fn unapplied_cross_device_copy_stage_is_removed() {
    let mut fs = operation_paths(&[
        ("/project", true),
        ("/mnt", true),
        ("/project/.a.kawi-move", false),
        ("/mnt/.b.txt.kawi-copy-1-0", false),
    ]);
    let journal = DirectoryJournal::prepared(vec![
        JournalOperation::Rename {
            from: JournalPath::from(PathBuf::from("/project/a.txt")),
            to: JournalPath::from(PathBuf::from("/project/.a.kawi-move")),
            stage: None,
        },
        JournalOperation::Rename {
            from: JournalPath::from(PathBuf::from("/project/.a.kawi-move")),
            to: JournalPath::from(PathBuf::from("/mnt/b.txt")),
            stage: Some(JournalPath::from(PathBuf::from(
                "/mnt/.b.txt.kawi-copy-1-0",
            ))),
        },
    ]);
    let report = recover(&journal, &HashSet::new(), &mut fs);
    assert!(report.errors.is_empty());
    assert_eq!(report.removed, 1);
    assert_eq!(report.restored, 1);
    assert!(fs.exists(std::path::Path::new("/project/a.txt")));
    assert!(!fs.exists(std::path::Path::new("/mnt/.b.txt.kawi-copy-1-0")));
    assert!(!fs.exists(std::path::Path::new("/mnt/b.txt")));
}

#[test]
fn unapplied_copy_stage_is_removed_without_removing_the_source() {
    let mut fs = operation_paths(&[
        ("/project", true),
        ("/mnt", true),
        ("/project/a.txt", false),
        ("/mnt/.b.txt.kawi-copy-0-0", false),
    ]);
    let journal = DirectoryJournal::prepared(vec![JournalOperation::Copy {
        from: JournalPath::from(PathBuf::from("/project/a.txt")),
        to: JournalPath::from(PathBuf::from("/mnt/b.txt")),
        stage: Some(JournalPath::from(PathBuf::from(
            "/mnt/.b.txt.kawi-copy-0-0",
        ))),
    }]);
    let report = recover(&journal, &HashSet::new(), &mut fs);
    assert!(report.errors.is_empty());
    assert_eq!(report.removed, 1);
    assert!(fs.exists(std::path::Path::new("/project/a.txt")));
    assert!(!fs.exists(std::path::Path::new("/mnt/.b.txt.kawi-copy-0-0")));
    assert!(!fs.exists(std::path::Path::new("/mnt/b.txt")));
}

#[test]
fn promotion_intent_with_a_remaining_stage_keeps_the_target() {
    let mut fs = operation_paths(&[
        ("/project", true),
        ("/mnt", true),
        ("/project/a.txt", false),
        ("/mnt/.b.txt.kawi-copy-0-0", false),
        ("/mnt/b.txt", false),
    ]);
    let journal = DirectoryJournal::prepared(vec![JournalOperation::Rename {
        from: JournalPath::from(PathBuf::from("/project/a.txt")),
        to: JournalPath::from(PathBuf::from("/mnt/b.txt")),
        stage: Some(JournalPath::from(PathBuf::from(
            "/mnt/.b.txt.kawi-copy-0-0",
        ))),
    }]);
    let report = recover(&journal, &HashSet::from([0]), &mut fs);
    assert!(report.errors.is_empty());
    assert_eq!(report.skipped.len(), 1);
    assert_eq!(report.removed, 1);
    assert!(fs.exists(std::path::Path::new("/project/a.txt")));
    assert!(fs.exists(std::path::Path::new("/mnt/b.txt")));
    assert!(!fs.exists(std::path::Path::new("/mnt/.b.txt.kawi-copy-0-0")));
}

#[test]
fn promoted_rename_with_a_missing_source_is_reported() {
    let mut fs = operation_paths(&[("/project", true), ("/mnt", true), ("/mnt/b.txt", false)]);
    let journal = DirectoryJournal::prepared(vec![JournalOperation::Rename {
        from: JournalPath::from(PathBuf::from("/project/a.txt")),
        to: JournalPath::from(PathBuf::from("/mnt/b.txt")),
        stage: Some(JournalPath::from(PathBuf::from(
            "/mnt/.b.txt.kawi-copy-0-0",
        ))),
    }]);
    let report = recover(&journal, &HashSet::from([0]), &mut fs);
    assert_eq!(report.errors.len(), 1);
    assert!(fs.exists(std::path::Path::new("/mnt/b.txt")));
}
