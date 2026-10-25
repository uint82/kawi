use anyhow::Result;

use kawi_git::{BranchListOptions, SwitchMode, SwitchOutcome};
use kawi_lsp::WatchedFileChange;

use super::super::{picker::PickerSession, App};
use super::actions::{branch_by_ref, branch_row};
use super::state::PendingSwitch;

const BRANCH_HINT: &str = "Enter switch; Ctrl-B create; Ctrl-D delete";

impl App {
    pub(crate) fn open_branch_picker(&mut self) -> Result<bool> {
        if self.git_unavailable() {
            return Ok(false);
        }
        let root = self.git_root().unwrap_or_default();
        match kawi_git::list_branches(&root, BranchListOptions::default()) {
            Ok(list) => {
                let rows = branch_rows(&list);
                let status = branch_status(&list);
                self.git_branch.branches = Some(list);
                self.picker = Some(PickerSession::git_branches(root, rows, Some(status)));
            }
            Err(error) => self.show_message(format!("Git branches failed: {error}")),
        }
        Ok(false)
    }

    pub(crate) fn switch_selected_branch(&mut self, full_ref: &str) -> Result<bool> {
        let Some(branch) = branch_by_ref(&self.git_branch, full_ref).cloned() else {
            self.show_message("Branch is no longer available".to_string());
            return Ok(false);
        };
        if branch.kind == kawi_git::BranchKind::Detached {
            self.show_message("Already detached at this commit".to_string());
            return Ok(false);
        }
        if branch.is_current {
            self.show_message(format!("Already on {}", branch.name));
            return Ok(false);
        }
        if let Some(path) = branch.worktree.as_deref() {
            self.show_message(format!(
                "Branch {} is checked out in {}",
                branch.name,
                path.display()
            ));
            return Ok(false);
        }
        self.perform_switch(branch, SwitchMode::Normal)
    }

    pub(crate) fn handle_switch_command(&mut self, target: &str) -> Result<bool> {
        if self.git_unavailable() {
            return Ok(false);
        }
        let root = self.git_root().unwrap_or_default();
        let list = match kawi_git::list_branches(&root, BranchListOptions::default()) {
            Ok(list) => list,
            Err(error) => {
                self.show_message(format!("Git branches failed: {error}"));
                return Ok(false);
            }
        };
        self.git_branch.branches = Some(list.clone());
        let target = short_ref(target);
        if let Some(branch) = list
            .branches
            .iter()
            .find(|branch| branch.name == target)
            .cloned()
        {
            return self.perform_switch(branch, SwitchMode::Normal);
        }
        self.perform_detached(target)
    }

    pub(crate) fn perform_switch(
        &mut self,
        branch: kawi_git::Branch,
        mode: SwitchMode,
    ) -> Result<bool> {
        let Some(root) = self.git_root() else {
            self.show_message("Not inside a git repository".to_string());
            return Ok(false);
        };
        self.sync_watched_files();
        let outcome = match kawi_git::switch(&root, &branch, mode) {
            Ok(outcome) => outcome,
            Err(error) => {
                self.show_message(format!("Git switch failed: {error}"));
                return Ok(false);
            }
        };
        match outcome {
            SwitchOutcome::Switched => {
                self.picker = None;
                self.show_message(format!("Switched to {}", branch.name));
                self.after_checkout();
            }
            SwitchOutcome::Refused(message) if is_worktree_refusal(&message) => {
                self.show_message(message);
            }
            SwitchOutcome::Refused(message) => {
                self.picker = None;
                self.git_branch.pending_switch = Some(PendingSwitch { branch });
                let message = super::super::compact_error(&message);
                self.show_message(format!(
                    "{message}  [s] stash and switch  [m] merge  [Esc] cancel"
                ));
            }
            SwitchOutcome::Failed(message) => {
                self.show_message(format!("Git switch failed: {message}"));
            }
        }
        Ok(false)
    }

    pub(crate) fn perform_detached(&mut self, target: &str) -> Result<bool> {
        let Some(root) = self.git_root() else {
            self.show_message("Not inside a git repository".to_string());
            return Ok(false);
        };
        self.sync_watched_files();
        let outcome = match kawi_git::switch_detached(&root, target) {
            Ok(outcome) => outcome,
            Err(error) => {
                self.show_message(format!("Git checkout failed: {error}"));
                return Ok(false);
            }
        };
        match outcome {
            SwitchOutcome::Switched => {
                self.picker = None;
                self.show_message(format!("Checked out {target} detached"));
                self.after_checkout();
            }
            SwitchOutcome::Refused(message) | SwitchOutcome::Failed(message) => {
                self.show_message(format!("Git checkout failed: {message}"));
            }
        }
        Ok(false)
    }

    pub(crate) fn after_checkout(&mut self) {
        self.reload_open_files();
        let files: Vec<_> = self
            .open_files()
            .into_iter()
            .map(|(_, path)| {
                let change = if path.exists() {
                    WatchedFileChange::Changed
                } else {
                    WatchedFileChange::Deleted
                };
                (path, change)
            })
            .collect();
        self.lsp.did_change_watched_files(&files);
        self.problems.mark_tasks_stale();
        self.mark_problems_changed();
        let _ = self.handle_git_refresh();
        self.refresh_git_status();
        self.sync_git();
    }
}

fn is_worktree_refusal(message: &str) -> bool {
    message.contains("checked out in") || message.contains("already checked out")
}

fn short_ref(target: &str) -> &str {
    target
        .strip_prefix("refs/heads/")
        .or_else(|| target.strip_prefix("refs/remotes/"))
        .unwrap_or(target)
}

fn branch_rows(list: &kawi_git::BranchList) -> Vec<kawi_picker::PickerRow> {
    let mut rows: Vec<kawi_picker::PickerRow> = list.branches.iter().map(branch_row).collect();
    if let Some(detached) = list.detached.as_ref() {
        rows.insert(0, branch_row(detached));
    }
    rows
}

fn branch_status(list: &kawi_git::BranchList) -> String {
    if list.unborn {
        return "Unborn branch; :git branch <name> creates the first branch".to_string();
    }
    if list.detached.is_some() {
        return format!("Detached HEAD; {BRANCH_HINT}");
    }
    BRANCH_HINT.to_string()
}
