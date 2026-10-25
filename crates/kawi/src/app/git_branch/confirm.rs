use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use kawi_git::{BranchListOptions, CreateOutcome, DeleteOutcome, SwitchMode};

use super::super::App;
use super::state::PendingDelete;

impl App {
    pub(crate) fn has_pending_branch_action(&self) -> bool {
        self.git_branch.pending_switch.is_some()
            || self.git_branch.pending_delete.is_some()
            || self.git_branch.pending_checkout.is_some()
    }

    pub(crate) fn handle_pending_branch_key(&mut self, key: KeyEvent) -> Result<bool> {
        if self.git_branch.pending_switch.is_some() {
            return self.handle_pending_switch(key);
        }
        if self.git_branch.pending_delete.is_some() {
            return self.handle_pending_delete(key);
        }
        if self.git_branch.pending_checkout.is_some() {
            return self.handle_pending_checkout(key);
        }
        Ok(false)
    }

    pub(crate) fn create_branch(&mut self, name: &str, base: Option<&str>) -> Result<bool> {
        if self.git_unavailable() {
            return Ok(false);
        }
        let root = self.git_root().unwrap_or_default();
        let unborn = self.git_branch.branches.as_ref().map_or_else(
            || kawi_git::is_unborn(&root).unwrap_or(false),
            |list| list.unborn,
        );
        let outcome = match kawi_git::create(&root, name, base, unborn) {
            Ok(outcome) => outcome,
            Err(error) => {
                self.show_message(format!("Git branch failed: {error}"));
                return Ok(false);
            }
        };
        match outcome {
            CreateOutcome::Created => {
                self.show_message(format!("Created branch {name}"));
                self.refresh_git_status();
            }
            CreateOutcome::Invalid(message) | CreateOutcome::Failed(message) => {
                self.show_message(format!("Git branch failed: {message}"));
            }
        }
        Ok(false)
    }

    pub(crate) fn delete_branch_ref(&mut self, full_ref: &str) -> Result<bool> {
        let Some(name) = super::actions::branch_by_ref(&self.git_branch, full_ref)
            .map(|branch| branch.name.clone())
        else {
            self.show_message("Branch is no longer available".to_string());
            return Ok(false);
        };
        self.delete_branch(&name, false)
    }

    pub(crate) fn delete_branch(&mut self, name: &str, force: bool) -> Result<bool> {
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
        let Some(branch) = list
            .branches
            .iter()
            .find(|branch| branch.name == name)
            .cloned()
        else {
            self.show_message(format!("Branch {name} not found"));
            return Ok(false);
        };
        self.git_branch.branches = Some(list);
        let outcome = match kawi_git::delete(&root, &branch, force) {
            Ok(outcome) => outcome,
            Err(error) => {
                self.show_message(format!("Git branch failed: {error}"));
                return Ok(false);
            }
        };
        match outcome {
            DeleteOutcome::Deleted => {
                self.show_message(format!("Deleted branch {name}"));
                self.refresh_git_status();
            }
            DeleteOutcome::NeedsForce { tip } => {
                self.git_branch.pending_delete = Some(PendingDelete { branch });
                self.show_message(format!(
                    "Branch {name} is not fully merged (tip {tip}); [y] delete anyway  [n] cancel"
                ));
            }
            DeleteOutcome::Refused(message) | DeleteOutcome::Failed(message) => {
                self.show_message(format!("Git branch failed: {message}"));
            }
        }
        Ok(false)
    }

    fn handle_pending_switch(&mut self, key: KeyEvent) -> Result<bool> {
        let mode = match key.code {
            KeyCode::Char('s') => SwitchMode::Stash,
            KeyCode::Char('m') => SwitchMode::Merge,
            KeyCode::Esc | KeyCode::Char('n') => {
                self.git_branch.pending_switch = None;
                self.clear_message();
                return Ok(false);
            }
            _ => return Ok(false),
        };
        let Some(pending) = self.git_branch.pending_switch.take() else {
            return Ok(false);
        };
        self.perform_switch(pending.branch, mode)
    }

    fn handle_pending_delete(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('y') => {}
            KeyCode::Esc | KeyCode::Char('n') => {
                self.git_branch.pending_delete = None;
                self.clear_message();
                return Ok(false);
            }
            _ => return Ok(false),
        }
        let Some(pending) = self.git_branch.pending_delete.take() else {
            return Ok(false);
        };
        self.delete_branch(&pending.branch.name, true)
    }
}
