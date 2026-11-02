use std::path::{Path, PathBuf};

use super::super::executor::{Outcome, PreparedPlan};
use super::super::fs::{DirectoryFs, RealFs};
use super::super::planner::Operation;
use crate::recovery::{DirectoryJournal, JournalOperation, JournalPath, RecoveryManager};

pub(super) fn create(prepared: &PreparedPlan<'_>) -> Result<DirectoryJournal, String> {
    let operations = prepared
        .plan()
        .operations
        .iter()
        .enumerate()
        .map(|(index, operation)| journal_operation(prepared, index, operation))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(DirectoryJournal::prepared(operations))
}

fn journal_operation(
    prepared: &PreparedPlan<'_>,
    index: usize,
    operation: &Operation,
) -> Result<JournalOperation, String> {
    match operation {
        Operation::Rename { from, to } => Ok(JournalOperation::Rename {
            from: JournalPath::from(from.clone()),
            to: JournalPath::from(to.clone()),
            stage: prepared
                .stage_path(index)
                .map(|stage| JournalPath::from(stage.to_path_buf())),
        }),
        Operation::Copy { from, to } => Ok(JournalOperation::Copy {
            from: JournalPath::from(from.clone()),
            to: JournalPath::from(to.clone()),
            stage: Some(stage_path(prepared, index, to)?),
        }),
        Operation::CreateDirectory(path) => Ok(JournalOperation::CreateDirectory {
            path: JournalPath::from(path.clone()),
        }),
        Operation::CreateFile(path) => Ok(JournalOperation::CreateFile {
            path: JournalPath::from(path.clone()),
        }),
        Operation::Trash(path) => {
            let staged = prepared
                .staged_path(index)
                .ok_or_else(|| format!("delete was not prepared for {}", path.display()))?;
            Ok(JournalOperation::Delete {
                path: JournalPath::from(path.clone()),
                staged: JournalPath::from(staged.to_path_buf()),
            })
        }
    }
}

fn stage_path(
    prepared: &PreparedPlan<'_>,
    index: usize,
    target: &Path,
) -> Result<JournalPath, String> {
    prepared
        .stage_path(index)
        .map(|stage| JournalPath::from(stage.to_path_buf()))
        .ok_or_else(|| format!("copy stage was not prepared for {}", target.display()))
}

pub(super) fn persist(
    manager: Option<&RecoveryManager>,
    journal: &DirectoryJournal,
) -> Result<bool, String> {
    let Some(manager) = manager else {
        return Ok(false);
    };
    manager.remove_directory_promotions()?;
    manager.save_directory_journal(journal)?;
    Ok(true)
}

pub(super) fn finish_success(
    manager: Option<&RecoveryManager>,
    journal: Option<&DirectoryJournal>,
    outcome: &Outcome,
    written: bool,
) -> Option<String> {
    let deferred = &outcome.deferred_deletes;
    if !written {
        return remove_deferred(deferred).err();
    }
    let Some(manager) = manager else {
        return Some("Directory recovery journal missing; deferred removals kept".to_string());
    };
    let Some(journal) = journal else {
        return Some("Directory recovery journal missing; deferred removals kept".to_string());
    };
    let committed = journal
        .clone()
        .committed(deferred.iter().cloned().map(JournalPath::from).collect());
    if let Err(error) = manager.save_directory_journal(&committed) {
        return finalize_without_journal(manager, deferred, &error);
    }
    if let Some(error) = remove_deferred(deferred).err() {
        return Some(format!("{error}; directory recovery journal kept"));
    }
    remove_recovery_files(manager)
        .err()
        .map(|error| format!("Cannot remove directory recovery journal: {error}"))
}

fn finalize_without_journal(
    manager: &RecoveryManager,
    deferred: &[PathBuf],
    commit_error: &str,
) -> Option<String> {
    if let Err(remove_error) = remove_recovery_files(manager) {
        return Some(format!(
            "Directory recovery journal could not be finalized: {commit_error}; {remove_error}; deferred removals kept"
        ));
    }
    remove_deferred(deferred).err()
}

fn remove_deferred(paths: &[PathBuf]) -> Result<(), String> {
    let mut fs = RealFs::new(true);
    for path in paths {
        if !fs.exists(path) {
            continue;
        }
        fs.remove_tree(path)
            .and_then(|()| fs.sync_parent(path))
            .map_err(|error| format!("cannot remove {}: {error}", path.display()))?;
    }
    Ok(())
}

fn remove_recovery_files(manager: &RecoveryManager) -> Result<(), String> {
    manager.remove_directory_journal()?;
    manager.remove_directory_promotions()
}

pub(super) fn finish_failure(
    manager: Option<&RecoveryManager>,
    outcome: &Outcome,
    written: bool,
) -> Option<String> {
    if !written {
        return None;
    }
    if !outcome.rollback_errors.is_empty() {
        return Some("Directory recovery journal kept for the next start".to_string());
    }
    let Some(manager) = manager else {
        return Some("Directory recovery journal kept for the next start".to_string());
    };
    remove_recovery_files(manager)
        .err()
        .map(|error| format!("Cannot remove directory recovery journal after rollback: {error}"))
}

#[cfg(test)]
mod tests;
