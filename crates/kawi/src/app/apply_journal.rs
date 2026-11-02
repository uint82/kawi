use std::path::Path;

use super::directory::complete_cleanups;
use super::directory::executor::PreparedPlan;
use super::directory::fs::RealFs;
use super::directory::planner::Operation;
use crate::recovery::{
    ApplyJournal, JournalCleanup, JournalOperation, JournalPath, RecoveryManager,
};

pub(in crate::app) fn create_for_plan(prepared: &PreparedPlan<'_>) -> Result<ApplyJournal, String> {
    let operations = prepared
        .plan()
        .operations
        .iter()
        .enumerate()
        .map(|(index, operation)| journal_operation(prepared, index, operation))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ApplyJournal::prepared(operations))
}

fn journal_operation(
    prepared: &PreparedPlan<'_>,
    index: usize,
    operation: &Operation,
) -> Result<JournalOperation, String> {
    match operation {
        Operation::Rename { from, to, force } => Ok(JournalOperation::Rename {
            from: JournalPath::from(from.clone()),
            to: JournalPath::from(to.clone()),
            stage: prepared
                .stage_path(index)
                .map(|stage| JournalPath::from(stage.to_path_buf())),
            force: *force,
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

pub(in crate::app) fn persist(
    manager: Option<&RecoveryManager>,
    journal: &ApplyJournal,
) -> Result<bool, String> {
    let Some(manager) = manager else {
        return Ok(false);
    };
    manager.remove_apply_promotions()?;
    manager.save_apply_journal(journal)?;
    Ok(true)
}

pub(in crate::app) fn finish_success(
    manager: Option<&RecoveryManager>,
    journal: Option<&ApplyJournal>,
    cleanups: &[JournalCleanup],
    written: bool,
) -> Option<String> {
    if !written {
        return cleanup_error(cleanups).err();
    }
    let Some(manager) = manager else {
        return Some("Apply recovery journal missing; deferred cleanup kept".to_string());
    };
    let Some(journal) = journal else {
        return Some("Apply recovery journal missing; deferred cleanup kept".to_string());
    };
    let committed = journal.clone().committed(cleanups.to_vec());
    if let Err(error) = manager.save_apply_journal(&committed) {
        return finalize_without_journal(manager, cleanups, &error);
    }
    if let Some(error) = cleanup_error(cleanups).err() {
        return Some(format!("{error}; apply recovery journal kept"));
    }
    remove_recovery_files(manager)
        .err()
        .map(|error| format!("Cannot remove apply recovery journal: {error}"))
}

fn finalize_without_journal(
    manager: &RecoveryManager,
    cleanups: &[JournalCleanup],
    commit_error: &str,
) -> Option<String> {
    if let Err(remove_error) = remove_recovery_files(manager) {
        return Some(format!(
            "Apply recovery journal could not be finalized: {commit_error}; {remove_error}; deferred cleanup kept"
        ));
    }
    cleanup_error(cleanups).err()
}

fn cleanup_error(cleanups: &[JournalCleanup]) -> Result<(), String> {
    let mut fs = RealFs::new(true);
    let report = complete_cleanups(cleanups, &mut fs);
    if report.errors.is_empty() {
        Ok(())
    } else {
        Err(report.errors.join("; "))
    }
}

fn remove_recovery_files(manager: &RecoveryManager) -> Result<(), String> {
    manager.remove_apply_journal()?;
    manager.remove_apply_promotions()
}

pub(in crate::app) fn finish_failure(
    manager: Option<&RecoveryManager>,
    rollback_clean: bool,
    written: bool,
) -> Option<String> {
    if !written {
        return None;
    }
    if !rollback_clean {
        return Some("Apply recovery journal kept for the next start".to_string());
    }
    let Some(manager) = manager else {
        return Some("Apply recovery journal kept for the next start".to_string());
    };
    remove_recovery_files(manager)
        .err()
        .map(|error| format!("Cannot remove apply recovery journal after rollback: {error}"))
}

#[cfg(test)]
mod tests;
