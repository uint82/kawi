use std::path::PathBuf;

use super::super::save::SaveAfter;
use super::diff::{diff, Change};
use super::executor::{applied_description, execute};
use super::fs::RealFs;
use super::permission_check::PermissionPhase;
use super::planner::Plan;
use super::session::{DirectoryEdit, PendingApply};
use crate::app::App;

impl App {
    pub(in crate::app) fn request_directory_save(
        &mut self,
        number: usize,
        force: bool,
        after: SaveAfter,
        visible: bool,
    ) -> bool {
        self.sync_current_directory();
        if self.reject_directory_save(number, visible) {
            return false;
        }
        let Some(changes) = self.pending_directory_changes(number) else {
            return false;
        };
        if changes.is_empty() {
            return self.finish_empty_directory_save(number, visible, after);
        }
        let started = self.start_directory_save_check(number, changes, force, after, visible);
        if visible {
            self.show_message(match started {
                Ok(()) => "Checking directory...".to_string(),
                Err(message) => message,
            });
        }
        false
    }

    fn reject_directory_save(&mut self, number: usize, visible: bool) -> bool {
        if self.reject_running_permission_check(visible) {
            return true;
        }
        if !self.directory_loading.is_loading(number) {
            return false;
        }
        if visible {
            self.show_message("Directory is still loading".to_string());
        }
        true
    }

    fn pending_directory_changes(&mut self, number: usize) -> Option<Vec<Change>> {
        match self.directory_changes(number) {
            Ok(changes) => Some(changes),
            Err(message) => {
                self.show_message(message);
                None
            }
        }
    }

    pub(super) fn directory_changes(&self, number: usize) -> Result<Vec<Change>, String> {
        let state = self
            .directory
            .get(number)
            .ok_or_else(|| "directory buffer is missing".to_string())?;
        diff(&state.root, &state.original, &state.lines)
    }

    fn finish_empty_directory_save(
        &mut self,
        number: usize,
        visible: bool,
        after: SaveAfter,
    ) -> bool {
        self.editor.mark_buffer_clean(number);
        if visible {
            self.show_message("No directory changes".to_string());
        }
        self.complete_save(after);
        true
    }

    pub(in crate::app) fn begin_directory_apply(&mut self, apply: PendingApply) -> bool {
        let renames = rename_requests(&apply.edits);
        let request = if renames.is_empty() {
            0
        } else {
            self.lsp.will_rename_files(&renames)
        };
        if request != 0 {
            self.pending_file_op = Some(super::super::file_ops::PendingFileOp::Directory(
                super::super::file_ops::PendingDirectoryOp { request, apply },
            ));
            self.show_message("Applying directory changes...".to_string());
            return false;
        }
        self.apply_directory_changes(apply)
    }

    pub(in crate::app) fn apply_directory_changes(&mut self, apply: PendingApply) -> bool {
        self.start_permission_check(apply, PermissionPhase::Apply, false)
    }

    pub(super) fn execute_directory_changes(
        &mut self,
        apply: PendingApply,
        permanent_delete: bool,
    ) -> bool {
        let changes = all_changes(&apply.edits);
        let mut fs = RealFs::new(permanent_delete);
        let outcome = execute(&apply.plan, &mut fs);
        let applied = &apply.plan.operations[..outcome.applied];
        for operation in applied {
            self.update_applied_operation(operation);
        }
        if outcome.error.is_none() {
            self.notify_lsp_changes(&changes);
            for edit in &apply.edits {
                self.editor.mark_buffer_clean(edit.number);
            }
        }
        self.refresh_after_directory_change();
        match outcome.error {
            Some(error) => self.finish_directory_apply_failure(&apply, error, outcome.applied),
            None => self.finish_directory_apply_success(&apply),
        }
    }

    pub(super) fn finish_directory_apply_failure(
        &mut self,
        apply: &PendingApply,
        error: String,
        applied: usize,
    ) -> bool {
        let reload_error = self.reload_directories(&apply.edits);
        let mut message = if applied == 0 {
            format!("{error}; no operations were applied")
        } else {
            format!(
                "{error}; applied {applied} operation(s): {}; no rollback attempted",
                applied_operations(&apply.plan, applied)
            )
        };
        if let Some(reload_error) = reload_error {
            message.push_str("; ");
            message.push_str(&reload_error);
        }
        self.show_message(message);
        false
    }

    fn finish_directory_apply_success(&mut self, apply: &PendingApply) -> bool {
        if apply.visible {
            self.show_message(apply_success_message(self, &apply.edits, &apply.plan));
        }
        let saved = if apply.save_files {
            self.finish_save_all_files(apply.after)
        } else {
            self.complete_save(apply.after);
            true
        };
        if !self.quit_pending {
            if let Some(error) = self.reload_directories(&apply.edits) {
                self.show_message(error);
            }
        }
        saved
    }

    fn reload_directories(&mut self, edits: &[DirectoryEdit]) -> Option<String> {
        let mut first_error = None;
        for edit in edits {
            if !self.directory.contains(edit.number) {
                self.editor.set_buffer_readonly(edit.number, false);
                continue;
            }
            if let Err(error) = self.start_directory_reload_after_save(edit.number) {
                self.editor.set_buffer_readonly(edit.number, false);
                first_error.get_or_insert(error);
            }
        }
        first_error
    }

    fn update_applied_operation(&mut self, operation: &super::planner::Operation) {
        use super::planner::Operation;
        match operation {
            Operation::Rename { from, to } => {
                let adoptions = self.editor.rewrite_buffer_paths(from, to);
                self.undo.move_prefix(from, to);
                self.problems.rewrite_paths(from, to);
                self.explorer.rename_path(from, to);
                self.directory.rewrite_path(from, to);
                for adoption in &adoptions {
                    self.record_saved_buffer_number(adoption.number);
                }
            }
            Operation::Trash(path) => {
                let numbers = super::super::file_ops::buffer_numbers_under(self, path);
                self.undo.remove_prefix(path);
                self.problems.remove_paths(path);
                self.explorer.remove_path(path);
                let directories = self.directory.remove_under(path);
                for buffer in numbers {
                    let _ = self.editor.close_buffer(buffer, true);
                }
                for buffer in directories {
                    self.directory_loading.forget(buffer);
                    let _ = self.editor.close_buffer(buffer, true);
                }
            }
            Operation::CreateFile(_) | Operation::CreateDirectory(_) | Operation::Copy { .. } => {}
        }
    }

    fn notify_lsp_changes(&mut self, changes: &[Change]) {
        let renames: Vec<(PathBuf, PathBuf)> = changes
            .iter()
            .filter_map(|change| match change {
                Change::Rename { from, to, .. } => Some((from.clone(), to.clone())),
                _ => None,
            })
            .collect();
        if !renames.is_empty() {
            self.lsp.did_rename_files(&renames);
        }
        let deletes: Vec<PathBuf> = changes
            .iter()
            .filter_map(|change| match change {
                Change::Delete { path, .. } => Some(path.clone()),
                _ => None,
            })
            .collect();
        if !deletes.is_empty() {
            self.lsp.did_delete_files(&deletes);
        }
    }

    pub(super) fn refresh_after_directory_change(&mut self) {
        self.lsp.refresh(&self.editor);
        self.sync_language_settings();
        self.sync_watched_files();
        self.refresh_git_status();
        self.explorer.refresh();
    }
}

fn applied_operations(plan: &Plan, applied: usize) -> String {
    plan.operations
        .iter()
        .take(applied)
        .map(applied_description)
        .collect::<Vec<_>>()
        .join(", ")
}

fn rename_requests(edits: &[DirectoryEdit]) -> Vec<(PathBuf, PathBuf)> {
    edits
        .iter()
        .flat_map(|edit| edit.changes.iter())
        .filter_map(|change| match change {
            Change::Rename { from, to, .. } => Some((from.clone(), to.clone())),
            _ => None,
        })
        .collect()
}

pub(super) fn all_changes(edits: &[DirectoryEdit]) -> Vec<Change> {
    edits
        .iter()
        .flat_map(|edit| edit.changes.iter().cloned())
        .collect()
}

fn apply_success_message(app: &App, edits: &[DirectoryEdit], plan: &Plan) -> String {
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
