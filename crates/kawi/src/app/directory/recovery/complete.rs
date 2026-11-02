use crate::recovery::DirectoryJournal;

use super::super::fs::DirectoryFs;
use super::report::{remove_tree, RecoveryReport};

pub(super) fn complete(journal: &DirectoryJournal, fs: &mut impl DirectoryFs) -> RecoveryReport {
    let mut report = RecoveryReport::default();
    for path in journal.deferred() {
        remove_tree(fs, path.as_path(), &mut report);
    }
    report
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::app::directory::test_support::FakeFs;
    use crate::recovery::JournalPath;

    #[test]
    fn committed_sources_are_removed() {
        let mut fs = FakeFs::new(&[("/project", true), ("/project/.a.kawi-move", false)]);
        let journal = DirectoryJournal::prepared(Vec::new()).committed(vec![JournalPath::from(
            PathBuf::from("/project/.a.kawi-move"),
        )]);
        let report = complete(&journal, &mut fs);
        assert!(report.errors.is_empty());
        assert_eq!(report.removed, 1);
        assert!(!fs.exists(std::path::Path::new("/project/.a.kawi-move")));
    }

    #[test]
    fn already_removed_sources_are_ignored() {
        let mut fs = FakeFs::new(&[("/project", true)]);
        let journal = DirectoryJournal::prepared(Vec::new()).committed(vec![JournalPath::from(
            PathBuf::from("/project/.a.kawi-move"),
        )]);
        let report = complete(&journal, &mut fs);
        assert!(report.errors.is_empty());
        assert_eq!(report.removed, 0);
    }
}
