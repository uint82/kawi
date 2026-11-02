use crate::recovery::{ApplyJournal, JournalCleanup};

use super::super::fs::DirectoryFs;
use super::report::{remove_tree, trash, RecoveryReport};

pub(super) fn complete(journal: &ApplyJournal, fs: &mut impl DirectoryFs) -> RecoveryReport {
    complete_cleanups(&journal.cleanups(), fs)
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
    use crate::app::directory::test_support::FakeFs;

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
