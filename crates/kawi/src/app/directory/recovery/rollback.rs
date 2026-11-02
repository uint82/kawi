use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::recovery::{ApplyJournal, JournalOperation, JournalPath};

use super::super::fs::DirectoryFs;
use super::report::{remove_directory, remove_file, remove_tree, RecoveryReport};

pub(super) fn recover(
    journal: &ApplyJournal,
    promotions: &HashSet<usize>,
    fs: &mut impl DirectoryFs,
) -> RecoveryReport {
    let rename_origins = rename_origins(journal);
    let mut report = RecoveryReport::default();
    for (index, operation) in journal.operations.iter().enumerate().rev() {
        recover_operation(
            index,
            operation,
            promotions,
            fs,
            &rename_origins,
            &mut report,
        );
    }
    report
}

fn rename_origins(journal: &ApplyJournal) -> HashMap<PathBuf, PathBuf> {
    journal
        .operations
        .iter()
        .filter_map(|operation| match operation {
            JournalOperation::Rename { from, to, .. } => {
                Some((to.as_path().to_path_buf(), from.as_path().to_path_buf()))
            }
            _ => None,
        })
        .collect()
}

fn recover_operation(
    index: usize,
    operation: &JournalOperation,
    promotions: &HashSet<usize>,
    fs: &mut impl DirectoryFs,
    rename_origins: &HashMap<PathBuf, PathBuf>,
    report: &mut RecoveryReport,
) {
    match operation {
        JournalOperation::Rename {
            from, to, stage, ..
        } => {
            if promotions.contains(&index) {
                rollback_promoted_rename(fs, from.as_path(), to.as_path(), stage, report);
                return;
            }
            remove_stage(fs, stage, report);
            restore_rename(fs, from.as_path(), to.as_path(), rename_origins, report);
        }
        JournalOperation::Copy { to, stage, .. } => rollback_copy(fs, to.as_path(), stage, report),
        JournalOperation::CreateDirectory { path } => remove_directory(fs, path.as_path(), report),
        JournalOperation::CreateFile { path } => remove_file(fs, path.as_path(), report),
        JournalOperation::Delete { path, staged } => {
            restore_delete(fs, path.as_path(), staged.as_path(), report)
        }
    }
}

fn rollback_promoted_rename(
    fs: &mut impl DirectoryFs,
    from: &Path,
    to: &Path,
    stage: &Option<JournalPath>,
    report: &mut RecoveryReport,
) {
    if !fs.exists(from) {
        report.errors.push(format!(
            "cannot restore {}: source is missing",
            from.display()
        ));
        return;
    }
    if stage
        .as_ref()
        .is_some_and(|stage| fs.exists(stage.as_path()))
    {
        remove_stage(fs, stage, report);
        if fs.exists(to) {
            report
                .skipped
                .push(format!("both {} and its copy stage exist", to.display()));
        }
        return;
    }
    remove_tree(fs, to, report);
}

fn rollback_copy(
    fs: &mut impl DirectoryFs,
    to: &Path,
    stage: &Option<JournalPath>,
    report: &mut RecoveryReport,
) {
    if stage
        .as_ref()
        .is_some_and(|stage| fs.exists(stage.as_path()))
    {
        remove_stage(fs, stage, report);
        if fs.exists(to) {
            report
                .skipped
                .push(format!("both {} and its copy stage exist", to.display()));
        }
        return;
    }
    remove_tree(fs, to, report);
}

fn remove_stage(
    fs: &mut impl DirectoryFs,
    stage: &Option<JournalPath>,
    report: &mut RecoveryReport,
) {
    if let Some(stage) = stage {
        remove_tree(fs, stage.as_path(), report);
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
mod tests;
