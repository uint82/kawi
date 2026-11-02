use std::path::Path;

use super::super::fs::DirectoryFs;

#[derive(Default)]
pub(in crate::app) struct RecoveryReport {
    pub(in crate::app) restored: usize,
    pub(in crate::app) removed: usize,
    pub(in crate::app) irreversible: Vec<String>,
    pub(in crate::app) skipped: Vec<String>,
    pub(in crate::app) errors: Vec<String>,
}

impl RecoveryReport {
    pub(super) fn is_incomplete(&self) -> bool {
        !self.errors.is_empty() || !self.skipped.is_empty()
    }
}

pub(super) fn remove_tree(fs: &mut impl DirectoryFs, path: &Path, report: &mut RecoveryReport) {
    if !fs.exists(path) {
        return;
    }
    match fs.remove_tree(path).and_then(|()| fs.sync_parent(path)) {
        Ok(()) => report.removed = report.removed.saturating_add(1),
        Err(error) => report
            .errors
            .push(format!("cannot remove {}: {error}", path.display())),
    }
}

pub(super) fn remove_directory(
    fs: &mut impl DirectoryFs,
    path: &Path,
    report: &mut RecoveryReport,
) {
    if !fs.exists(path) {
        return;
    }
    if !fs.is_dir(path) {
        report
            .skipped
            .push(format!("{} is not a directory", path.display()));
        return;
    }
    match fs
        .remove_directory(path)
        .and_then(|()| fs.sync_parent(path))
    {
        Ok(()) => report.removed = report.removed.saturating_add(1),
        Err(error) => report
            .errors
            .push(format!("cannot remove {}: {error}", path.display())),
    }
}

pub(super) fn trash(
    fs: &mut impl DirectoryFs,
    staged: &Path,
    original: &Path,
    report: &mut RecoveryReport,
) {
    if !fs.exists(staged) {
        return;
    }
    match fs
        .trash(staged, original)
        .and_then(|()| fs.sync_parent(staged))
    {
        Ok(()) => report.removed = report.removed.saturating_add(1),
        Err(error) => report
            .errors
            .push(format!("cannot trash {}: {error}", original.display())),
    }
}

pub(super) fn remove_file(fs: &mut impl DirectoryFs, path: &Path, report: &mut RecoveryReport) {
    if !fs.exists(path) {
        return;
    }
    match fs.remove_file(path).and_then(|()| fs.sync_parent(path)) {
        Ok(()) => report.removed = report.removed.saturating_add(1),
        Err(error) => report
            .errors
            .push(format!("cannot remove {}: {error}", path.display())),
    }
}
