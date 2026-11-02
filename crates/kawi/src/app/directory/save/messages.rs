use super::super::executor::{applied_description, Outcome};
use super::super::planner::Plan;
use super::super::session::DirectoryEdit;
use crate::app::App;

pub(super) fn failure_message(plan: &Plan, outcome: &Outcome) -> String {
    let mut message = outcome.error.clone().unwrap_or_default();
    if outcome.completed == 0 {
        message.push_str("; no operations were applied");
        return message;
    }
    if outcome.rollback_errors.is_empty() {
        let rolled_back = outcome.completed.saturating_sub(outcome.remaining.len());
        if rolled_back > 0 {
            message.push_str(&format!("; rolled back {rolled_back} operation(s)"));
        }
    }
    if !outcome.remaining.is_empty() {
        message.push_str("; irreversible operations remain: ");
        message.push_str(&operations_at(plan, &outcome.remaining));
    }
    if !outcome.rollback_errors.is_empty() {
        message.push_str("; rollback incomplete: ");
        message.push_str(&outcome.rollback_errors.join("; "));
    }
    message
}

pub(super) fn apply_success_message(app: &App, edits: &[DirectoryEdit], plan: &Plan) -> String {
    if let [edit] = edits {
        let root = app
            .directory
            .get(edit.number)
            .map_or_else(String::new, |state| state.root_display());
        return format!("Applied {} change(s) to {root}", plan.changes);
    }
    format!(
        "Applied {} change(s) to {} directories",
        plan.changes,
        edits.len()
    )
}

fn operations_at(plan: &Plan, indices: &[usize]) -> String {
    indices
        .iter()
        .filter_map(|index| plan.operations.get(*index))
        .map(applied_description)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::app::directory::planner::Operation;

    fn plan(operations: Vec<Operation>) -> Plan {
        Plan {
            operations,
            changes: 1,
        }
    }

    fn outcome(completed: usize, remaining: Vec<usize>) -> Outcome {
        Outcome {
            completed,
            remaining,
            rollback_errors: Vec::new(),
            error: Some("operation failed".to_string()),
        }
    }

    #[test]
    fn failure_message_reports_a_complete_rollback() {
        let plan = plan(vec![Operation::CreateFile(PathBuf::from("/project/a.txt"))]);
        let message = failure_message(&plan, &outcome(1, Vec::new()));
        assert!(message.contains("operation failed"));
        assert!(message.contains("rolled back 1 operation(s)"));
    }

    #[test]
    fn failure_message_lists_irreversible_operations() {
        let plan = plan(vec![Operation::Trash(PathBuf::from("/project/a.txt"))]);
        let message = failure_message(&plan, &outcome(1, vec![0]));
        assert!(message.contains("irreversible operations remain: removed /project/a.txt"));
    }

    #[test]
    fn failure_message_reports_incomplete_rollbacks() {
        let plan = plan(vec![Operation::CreateFile(PathBuf::from("/project/a.txt"))]);
        let mut outcome = outcome(1, Vec::new());
        outcome.rollback_errors = vec!["cannot remove /project/a.txt".to_string()];
        let message = failure_message(&plan, &outcome);
        assert!(message.contains("rollback incomplete"));
        assert!(message.contains("cannot remove /project/a.txt"));
    }
}
