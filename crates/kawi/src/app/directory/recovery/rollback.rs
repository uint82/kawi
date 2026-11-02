use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::recovery::{DirectoryJournal, JournalOperation};

use super::super::fs::DirectoryFs;

#[derive(Default)]
pub(super) struct RecoveryReport {
    pub(super) restored: usize,
    pub(super) removed: usize,
    pub(super) irreversible: Vec<String>,
    pub(super) skipped: Vec<String>,
    pub(super) errors: Vec<String>,
}

impl RecoveryReport {
    pub(super) fn is_incomplete(&self) -> bool {
        !self.errors.is_empty() || !self.skipped.is_empty()
    }
}

pub(super) fn recover(journal: &DirectoryJournal, fs: &mut impl DirectoryFs) -> RecoveryReport {
    let rename_origins = rename_origins(journal);
    let mut report = RecoveryReport::default();
    for operation in journal.operations.iter().rev() {
        recover_operation(operation, fs, &rename_origins, &mut report);
    }
    report
}

fn rename_origins(journal: &DirectoryJournal) -> HashMap<PathBuf, PathBuf> {
    journal
        .operations
        .iter()
        .filter_map(|operation| match operation {
            JournalOperation::Rename { from, to } => {
                Some((to.as_path().to_path_buf(), from.as_path().to_path_buf()))
            }
            _ => None,
        })
        .collect()
}

fn recover_operation(
    operation: &JournalOperation,
    fs: &mut impl DirectoryFs,
    rename_origins: &HashMap<PathBuf, PathBuf>,
    report: &mut RecoveryReport,
) {
    match operation {
        JournalOperation::Rename { from, to } => {
            restore_rename(fs, from.as_path(), to.as_path(), rename_origins, report)
        }
        JournalOperation::Copy { to, .. } => remove_tree(fs, to.as_path(), report),
        JournalOperation::CreateDirectory { path } => remove_directory(fs, path.as_path(), report),
        JournalOperation::CreateFile { path } => remove_file(fs, path.as_path(), report),
        JournalOperation::Delete { path, staged } => {
            restore_delete(fs, path.as_path(), staged.as_path(), report)
        }
    }
}

fn restore_rename(
    fs: &mut impl DirectoryFs,
    from: &Path,
    to: &Path,
    rename_origins: &HashMap<PathBuf, PathBuf>,
    report: &mut RecoveryReport,
) {
    if fs.exists(from) {
        if fs.exists(to) {
            report.skipped.push(format!(
                "both {} and {} exist",
                from.display(),
                to.display()
            ));
        }
        return;
    }
    if !fs.exists(to) {
        return;
    }
    if let Some(origin) = rename_origins.get(from) {
        if fs.exists(origin) {
            report.skipped.push(format!(
                "{} still exists; left {} in place",
                origin.display(),
                to.display()
            ));
            return;
        }
    }
    match fs.rename(to, from) {
        Ok(()) => report.restored = report.restored.saturating_add(1),
        Err(error) => report
            .errors
            .push(format!("cannot restore {}: {error}", from.display())),
    }
}

fn remove_tree(fs: &mut impl DirectoryFs, path: &Path, report: &mut RecoveryReport) {
    if !fs.exists(path) {
        return;
    }
    match fs.remove_tree(path) {
        Ok(()) => report.removed = report.removed.saturating_add(1),
        Err(error) => report
            .errors
            .push(format!("cannot remove {}: {error}", path.display())),
    }
}

fn remove_directory(fs: &mut impl DirectoryFs, path: &Path, report: &mut RecoveryReport) {
    if !fs.exists(path) {
        return;
    }
    if !fs.is_dir(path) {
        report
            .skipped
            .push(format!("{} is not a directory", path.display()));
        return;
    }
    match fs.remove_directory(path) {
        Ok(()) => report.removed = report.removed.saturating_add(1),
        Err(error) => report
            .errors
            .push(format!("cannot remove {}: {error}", path.display())),
    }
}

fn remove_file(fs: &mut impl DirectoryFs, path: &Path, report: &mut RecoveryReport) {
    if !fs.exists(path) {
        return;
    }
    match fs.remove_file(path) {
        Ok(()) => report.removed = report.removed.saturating_add(1),
        Err(error) => report
            .errors
            .push(format!("cannot remove {}: {error}", path.display())),
    }
}

fn restore_delete(
    fs: &mut impl DirectoryFs,
    path: &Path,
    staged: &Path,
    report: &mut RecoveryReport,
) {
    if fs.exists(staged) {
        if fs.exists(path) {
            report
                .skipped
                .push(format!("both {} and its staged copy exist", path.display()));
            return;
        }
        match fs.rename(staged, path) {
            Ok(()) => report.restored = report.restored.saturating_add(1),
            Err(error) => report
                .errors
                .push(format!("cannot restore {}: {error}", path.display())),
        }
        return;
    }
    if !fs.exists(path) {
        report.irreversible.push(path.display().to_string());
    }
}

#[cfg(test)]
mod tests {
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
            },
            JournalOperation::Rename {
                from: JournalPath::from(PathBuf::from("/project/.a.kawi-move")),
                to: JournalPath::from(PathBuf::from("/project/b.txt")),
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
            },
        ]);
        let report = recover(&journal, &mut fs);
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
        let report = recover(&journal, &mut fs);
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
            },
            JournalOperation::Rename {
                from: JournalPath::from(PathBuf::from("/project/.a.kawi-move")),
                to: JournalPath::from(PathBuf::from("/project/b.txt")),
            },
        ]);
        let report = recover(&journal, &mut fs);
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
        }]);
        let report = recover(&journal, &mut fs);
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.restored, 0);
        assert!(fs.exists(std::path::Path::new("/project/a.txt")));
        assert!(fs.exists(std::path::Path::new("/project/.a.kawi-move")));
    }
}
