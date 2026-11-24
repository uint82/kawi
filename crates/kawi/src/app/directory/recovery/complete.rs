use std::path::Path;

use crate::recovery::{ApplyJournal, JournalCleanup, JournalOperation};

use super::super::fs::DirectoryFs;
use super::report::{remove_tree, trash, RecoveryReport};

pub(super) fn complete(journal: &ApplyJournal, fs: &mut impl DirectoryFs) -> RecoveryReport {
    complete_cleanups(&journal.cleanups(), fs)
}

pub(super) fn finish_prepared_deletes(
    journal: &ApplyJournal,
    fs: &mut impl DirectoryFs,
) -> RecoveryReport {
    let mut report = RecoveryReport::default();
    for operation in &journal.operations {
        if let JournalOperation::Delete { path, staged } = operation {
            finish_prepared_delete(path.as_path(), staged.as_path(), fs, &mut report);
        }
    }
    report
}

fn finish_prepared_delete(
    original: &Path,
    staged: &Path,
    fs: &mut impl DirectoryFs,
    report: &mut RecoveryReport,
) {
    if !fs.exists(staged) {
        return;
    }
    if fs.exists(original) {
        report.skipped.push(format!(
            "{} and {} both exist",
            original.display(),
            staged.display()
        ));
        return;
    }
    trash(fs, staged, original, report);
}

pub(in crate::app) fn complete_cleanups(
    cleanups: &[JournalCleanup],
    fs: &mut impl DirectoryFs,
) -> RecoveryReport {
    let mut report = RecoveryReport::default();
    for cleanup in cleanups {
        complete_cleanup(cleanup, fs, &mut report);
    }
    report
}

fn complete_cleanup(
    cleanup: &JournalCleanup,
    fs: &mut impl DirectoryFs,
    report: &mut RecoveryReport,
) {
    match cleanup {
        JournalCleanup::Remove { path } => remove_tree(fs, path.as_path(), report),
        JournalCleanup::Trash { staged, original } => {
            trash(fs, staged.as_path(), original.as_path(), report)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::app::directory::test_support::{Call, FakeFs};
    use crate::recovery::JournalPath;

    #[test]
    fn committed_sources_are_removed() {
        let mut fs = FakeFs::new(&[("/project", true), ("/project/.a.kawi-move", false)]);
        let journal = ApplyJournal::prepared(Vec::new()).committed(vec![JournalCleanup::remove(
            PathBuf::from("/project/.a.kawi-move"),
        )]);
        let report = complete(&journal, &mut fs);
        assert!(report.errors.is_empty());
        assert_eq!(report.removed, 1);
        assert!(!fs.exists(std::path::Path::new("/project/.a.kawi-move")));
    }

    #[test]
    fn committed_delete_stages_are_trashed() {
        let mut fs = FakeFs::new(&[("/project", true), ("/project/.a.kawi-delete", false)]);
        let journal = ApplyJournal::prepared(Vec::new()).committed(vec![JournalCleanup::trash(
            PathBuf::from("/project/.a.kawi-delete"),
            PathBuf::from("/project/a.txt"),
        )]);
        let report = complete(&journal, &mut fs);
        assert!(report.errors.is_empty());
        assert_eq!(report.removed, 1);
        assert!(!fs.exists(std::path::Path::new("/project/.a.kawi-delete")));
    }

    #[test]
    fn prepared_delete_stages_are_trashed() {
        let mut fs = FakeFs::new(&[("/project", true), ("/project/.a.kawi-delete", false)]);
        let journal = ApplyJournal::prepared(vec![JournalOperation::Delete {
            path: JournalPath::from(PathBuf::from("/project/a.txt")),
            staged: JournalPath::from(PathBuf::from("/project/.a.kawi-delete")),
        }]);
        let report = finish_prepared_deletes(&journal, &mut fs);
        assert!(report.errors.is_empty());
        assert_eq!(report.removed, 1);
        assert_eq!(fs.calls, [Call::Trash(PathBuf::from("/project/a.txt"))]);
        assert!(!fs.exists(std::path::Path::new("/project/.a.kawi-delete")));
    }

    #[test]
    fn prepared_delete_stages_are_skipped_when_the_original_exists() {
        let mut fs = FakeFs::new(&[
            ("/project", true),
            ("/project/a.txt", false),
            ("/project/.a.kawi-delete", false),
        ]);
        let journal = ApplyJournal::prepared(vec![JournalOperation::Delete {
            path: JournalPath::from(PathBuf::from("/project/a.txt")),
            staged: JournalPath::from(PathBuf::from("/project/.a.kawi-delete")),
        }]);
        let report = finish_prepared_deletes(&journal, &mut fs);
        assert_eq!(report.skipped.len(), 1);
        assert!(fs.calls.is_empty());
        assert!(fs.exists(std::path::Path::new("/project/.a.kawi-delete")));
    }

    #[test]
    fn prepared_delete_trash_failures_are_reported() {
        let mut fs =
            FakeFs::new(&[("/project", true), ("/project/.a.kawi-delete", false)]).fail_at(0);
        let journal = ApplyJournal::prepared(vec![JournalOperation::Delete {
            path: JournalPath::from(PathBuf::from("/project/a.txt")),
            staged: JournalPath::from(PathBuf::from("/project/.a.kawi-delete")),
        }]);
        let report = finish_prepared_deletes(&journal, &mut fs);
        assert_eq!(report.errors.len(), 1);
        assert!(report.errors[0].contains("cannot trash /project/a.txt"));
        assert!(fs.exists(std::path::Path::new("/project/.a.kawi-delete")));
    }

    #[test]
    fn missing_prepared_delete_stages_are_ignored() {
        let mut fs = FakeFs::new(&[("/project", true)]);
        let journal = ApplyJournal::prepared(vec![JournalOperation::Delete {
            path: JournalPath::from(PathBuf::from("/project/a.txt")),
            staged: JournalPath::from(PathBuf::from("/project/.a.kawi-delete")),
        }]);
        let report = finish_prepared_deletes(&journal, &mut fs);
        assert_eq!(report.removed, 0);
        assert!(report.errors.is_empty());
        assert!(fs.calls.is_empty());
    }

    #[test]
    fn already_removed_sources_are_ignored() {
        let mut fs = FakeFs::new(&[("/project", true)]);
        let journal = ApplyJournal::prepared(Vec::new()).committed(vec![JournalCleanup::remove(
            PathBuf::from("/project/.a.kawi-move"),
        )]);
        let report = complete(&journal, &mut fs);
        assert!(report.errors.is_empty());
        assert_eq!(report.removed, 0);
    }
}
