use super::super::executor::{Outcome, PreparedPlan};
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
        }),
        Operation::Copy { from, to } => Ok(JournalOperation::Copy {
            from: JournalPath::from(from.clone()),
            to: JournalPath::from(to.clone()),
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

pub(super) fn persist(
    manager: Option<&RecoveryManager>,
    journal: &DirectoryJournal,
) -> Result<bool, String> {
    let Some(manager) = manager else {
        return Ok(false);
    };
    manager.save_directory_journal(journal)?;
    Ok(true)
}

pub(super) fn finish_success(
    manager: Option<&RecoveryManager>,
    journal: &DirectoryJournal,
    written: bool,
) -> Option<String> {
    if !written {
        return None;
    }
    let manager = manager?;
    if let Err(commit_error) = manager.save_directory_journal(&journal.clone().committed()) {
        return match manager.remove_directory_journal() {
            Ok(()) => None,
            Err(remove_error) => Some(format!(
                "Directory recovery journal could not be finalized: {commit_error}; {remove_error}"
            )),
        };
    }
    manager
        .remove_directory_journal()
        .err()
        .map(|error| format!("Cannot remove directory recovery journal: {error}"))
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
    manager?
        .remove_directory_journal()
        .err()
        .map(|error| format!("Cannot remove directory recovery journal after rollback: {error}"))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::mpsc;

    use super::*;
    use crate::app::directory::planner::Plan;
    use crate::app::directory::test_support::FakeFs;
    use crate::recovery::{RecoveryManager, RecoveryStart, WorkspaceStore};
    use crate::test_support::temp_dir;

    fn manager(root: &std::path::Path) -> RecoveryManager {
        let (wake, _wake) = mpsc::channel();
        match RecoveryManager::start(root, root, wake) {
            RecoveryStart::Active { manager, .. } => *manager,
            RecoveryStart::Disabled(warning) => panic!("recovery disabled: {warning}"),
        }
    }

    fn prepared(plan: &Plan) -> PreparedPlan<'_> {
        let fs = FakeFs::new(&[("/project", true), ("/project/a.txt", false)]);
        PreparedPlan::new(plan, &fs).unwrap()
    }

    fn plan() -> Plan {
        Plan {
            operations: vec![Operation::Trash(PathBuf::from("/project/a.txt"))],
            changes: 1,
        }
    }

    #[test]
    fn journals_are_persisted_committed_and_removed() {
        let root = temp_dir("directory_journal_lifecycle");
        let manager = manager(&root);
        let plan = plan();
        let prepared = prepared(&plan);
        let journal = create(&prepared).unwrap();
        assert!(matches!(
            journal.operations.first(),
            Some(JournalOperation::Delete { .. })
        ));
        assert!(persist(Some(&manager), &journal).unwrap());
        let store = WorkspaceStore::create(&root, &root).unwrap();
        assert_eq!(
            store.load_directory_journal().unwrap(),
            Some(journal.clone())
        );
        assert!(finish_success(Some(&manager), &journal, true).is_none());
        assert_eq!(store.load_directory_journal().unwrap(), None);
        drop(manager);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn completed_rollbacks_remove_the_journal() {
        let root = temp_dir("directory_journal_completed_rollback");
        let manager = manager(&root);
        let plan = plan();
        let prepared = prepared(&plan);
        let journal = create(&prepared).unwrap();
        assert!(persist(Some(&manager), &journal).unwrap());
        let outcome = Outcome {
            completed: 1,
            remaining: Vec::new(),
            rollback_errors: Vec::new(),
            deferred_deletes: Vec::new(),
            error: Some("failed".to_string()),
        };
        assert!(finish_failure(Some(&manager), &outcome, true).is_none());
        let store = WorkspaceStore::create(&root, &root).unwrap();
        assert_eq!(store.load_directory_journal().unwrap(), None);
        drop(manager);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_rollback_keeps_the_journal_for_the_next_start() {
        let root = temp_dir("directory_journal_failed_rollback");
        let manager = manager(&root);
        let plan = plan();
        let prepared = prepared(&plan);
        let journal = create(&prepared).unwrap();
        assert!(persist(Some(&manager), &journal).unwrap());
        let outcome = Outcome {
            completed: 1,
            remaining: Vec::new(),
            rollback_errors: vec!["cannot restore".to_string()],
            deferred_deletes: Vec::new(),
            error: Some("failed".to_string()),
        };
        let warning = finish_failure(Some(&manager), &outcome, true).expect("journal warning");
        assert!(warning.contains("kept"));
        let store = WorkspaceStore::create(&root, &root).unwrap();
        assert_eq!(store.load_directory_journal().unwrap(), Some(journal));
        drop(manager);
        std::fs::remove_dir_all(root).unwrap();
    }
}
