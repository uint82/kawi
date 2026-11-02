use std::path::Path;

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
