use std::path::PathBuf;

use super::super::save::SaveAfter;
use super::diff::{diff, Change};
use super::executor::execute;
use super::fs::RealFs;
use super::planner::Plan;
use super::session::PendingApply;
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
        if self.directory_loading.is_loading(number) {
            if visible {
                self.show_message("Directory is still loading".to_string());
            }
            return false;
        }
        let Some(changes) = self.pending_directory_changes(number) else {
            return false;
        };
        if changes.is_empty() {
            return self.finish_empty_directory_save(number, visible, after);
        }
        match self.start_directory_save_check(number, changes, force, after, visible) {
            Ok(()) => {
                if visible {
                    self.show_message("Checking directory...".to_string());
                }
            }
            Err(message) => {
                if visible {
                    self.show_message(message);
                }
            }
        }
        false
    }

    fn pending_directory_changes(&mut self, number: usize) -> Option<Vec<Change>> {
        let state = self.directory.get(number)?;
        match diff(&state.root, &state.original, &state.lines) {
            Ok(changes) => Some(changes),
            Err(message) => {
                self.show_message(message);
                None
            }
        }
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

    pub(in crate::app) fn begin_directory_apply(
        &mut self,
        number: usize,
        changes: Vec<Change>,
        plan: Plan,
        after: SaveAfter,
        visible: bool,
    ) -> bool {
        let renames: Vec<(PathBuf, PathBuf)> = changes
            .iter()
            .filter_map(|change| match change {
                Change::Rename { from, to, .. } => Some((from.clone(), to.clone())),
                _ => None,
            })
            .collect();
        let request = if renames.is_empty() {
            0
        } else {
            self.lsp.will_rename_files(&renames)
        };
        if request != 0 {
            self.pending_file_op = Some(super::super::file_ops::PendingFileOp::Directory(
                super::super::file_ops::PendingDirectoryOp {
                    request,
                    apply: PendingApply {
                        number,
                        changes,
                        plan,
                        after,
                        visible,
                    },
                },
            ));
            self.show_message("Applying directory changes...".to_string());
            return false;
        }
        self.apply_directory_changes(number, changes, plan, after, visible)
    }

    pub(in crate::app) fn apply_directory_changes(
        &mut self,
        number: usize,
        changes: Vec<Change>,
        plan: Plan,
        after: SaveAfter,
        visible: bool,
    ) -> bool {
        let mut fs = RealFs::new(!self.directory_options.trash);
        let outcome = execute(&plan, &mut fs);
        let applied = &plan.operations[..outcome.applied];
        for operation in applied {
            self.update_applied_operation(operation);
        }
        if outcome.error.is_none() {
            self.notify_lsp_changes(&changes);
            self.editor.mark_buffer_clean(number);
        }
        self.refresh_after_directory_change();
        match outcome.error {
            Some(error) => self.finish_directory_apply_failure(number, error, outcome.applied),
            None => self.finish_directory_apply_success(number, plan, after, visible),
        }
    }

    fn finish_directory_apply_failure(
        &mut self,
        number: usize,
        error: String,
        applied: usize,
    ) -> bool {
        if let Err(reload_error) = self.start_directory_reload_after_save(number) {
            self.editor.set_buffer_readonly(number, false);
            self.show_message(format!(
                "{error}; applied {applied} operation(s); {reload_error}"
            ));
            return false;
        }
        self.show_message(format!("{error}; applied {applied} operation(s)"));
        false
    }

    fn finish_directory_apply_success(
        &mut self,
        number: usize,
        plan: Plan,
        after: SaveAfter,
        visible: bool,
    ) -> bool {
        if visible {
            self.show_message(format!(
                "Applied {} change(s) to {}",
                plan.changes,
                self.directory
                    .get(number)
                    .map_or_else(String::new, |state| state.root_display())
            ));
        }
        self.complete_save(after);
        if !self.quit_pending {
            if let Err(error) = self.start_directory_reload_after_save(number) {
                self.editor.set_buffer_readonly(number, false);
                self.show_message(error);
            }
        }
        true
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

    fn refresh_after_directory_change(&mut self) {
        self.lsp.refresh(&self.editor);
        self.sync_language_settings();
        self.sync_watched_files();
        self.refresh_git_status();
        self.explorer.refresh();
    }
}
