use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use super::diff::Change;
use super::executor::Outcome;
use super::fs::RealFs;
use super::permissions;
use super::planner::Operation;
use super::save::all_changes;
use super::session::PendingApply;
use crate::app::App;

struct PermissionTask {
    changes: Vec<Change>,
    operations: Vec<Operation>,
    permanent_delete: bool,
}

impl PermissionTask {
    fn spawn(self, sender: Sender<Result<(), String>>, wake: Sender<()>) -> std::io::Result<()> {
        thread::Builder::new()
            .name("kawi-permissions".to_string())
            .spawn(move || self.run(sender, wake))
            .map(|_| ())
    }

    fn run(self, sender: Sender<Result<(), String>>, wake: Sender<()>) {
        let fs = RealFs::new(self.permanent_delete);
        let checked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            permissions::preflight(&self.changes, &self.operations, &fs)
        }))
        .unwrap_or_else(|_| Err("Permission check stopped unexpectedly".to_string()));
        let _ = sender.send(checked);
        let _ = wake.send(());
    }
}

pub(in crate::app) struct PendingPermissionCheck {
    apply: PendingApply,
    phase: PermissionPhase,
    confirm: bool,
    permanent_delete: bool,
    result: Receiver<Result<(), String>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PermissionPhase {
    Plan,
    Apply,
}

impl App {
    pub(super) fn start_permission_check(
        &mut self,
        apply: PendingApply,
        phase: PermissionPhase,
        confirm: bool,
    ) -> bool {
        if self.pending_permission.is_some() {
            return self
                .cancel_permission_start(&apply, "Permission check is still running".to_string());
        }
        let (permanent_delete, result) = match self.spawn_permission_task(&apply) {
            Ok(check) => check,
            Err(message) => return self.cancel_permission_start(&apply, message),
        };
        if phase == PermissionPhase::Plan && apply.visible {
            self.show_message("Checking permissions...".to_string());
        }
        self.pending_permission = Some(PendingPermissionCheck {
            apply,
            phase,
            confirm,
            permanent_delete,
            result,
        });
        false
    }

    fn cancel_permission_start(&mut self, apply: &PendingApply, message: String) -> bool {
        self.cancel_directory_apply(&apply.edits);
        self.show_message(message);
        false
    }

    fn spawn_permission_task(
        &self,
        apply: &PendingApply,
    ) -> Result<(bool, Receiver<Result<(), String>>), String> {
        let permanent_delete = !self.directory_options.trash;
        let task = PermissionTask {
            changes: all_changes(&apply.edits),
            operations: apply.plan.operations.clone(),
            permanent_delete,
        };
        let (sender, result) = mpsc::channel();
        task.spawn(sender, self.events.wake())
            .map(|()| (permanent_delete, result))
            .map_err(|error| format!("Cannot start permission check: {error}"))
    }

    pub(in crate::app) fn poll_permission_check(&mut self) -> bool {
        let Some(pending) = self.pending_permission.as_ref() else {
            return false;
        };
        let result = match pending.result.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => {
                Err("Permission check stopped unexpectedly".to_string())
            }
        };
        let pending = self
            .pending_permission
            .take()
            .expect("pending permission check exists");
        self.finish_permission_check(pending, result)
    }

    fn finish_permission_check(
        &mut self,
        pending: PendingPermissionCheck,
        result: Result<(), String>,
    ) -> bool {
        match (pending.phase, result) {
            (PermissionPhase::Plan, Ok(())) if pending.confirm => {
                self.confirm_directory_apply(pending.apply);
                true
            }
            (PermissionPhase::Plan, Ok(())) => self.begin_directory_apply(pending.apply),
            (PermissionPhase::Plan, Err(error)) => {
                self.cancel_directory_apply(&pending.apply.edits);
                self.show_message(error);
                true
            }
            (PermissionPhase::Apply, Ok(())) => {
                self.execute_directory_changes(pending.apply, pending.permanent_delete)
            }
            (PermissionPhase::Apply, Err(error)) => {
                self.refresh_after_directory_change();
                self.finish_directory_apply_failure(&pending.apply, &Outcome::failed(error))
            }
        }
    }

    pub(in crate::app) fn is_checking_permissions(&self) -> bool {
        self.pending_permission.is_some()
    }

    pub(super) fn reject_running_permission_check(&mut self, visible: bool) -> bool {
        if !self.is_checking_permissions() {
            return false;
        }
        if visible {
            self.show_message("Permission check is still running".to_string());
        }
        true
    }

    #[cfg(test)]
    pub(in crate::app) fn has_pending_permission_check(&self) -> bool {
        self.pending_permission.is_some()
    }
}
