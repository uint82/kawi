use std::path::PathBuf;

use super::super::save::SaveAfter;
use super::diff::{diff, Change};
use super::entry::EntryKind;
use super::executor::execute;
use super::fs::RealFs;
use super::open::directory_changed;
use super::planner::{plan, Plan};
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
        let Some(state) = self.directory.get(number) else {
            return false;
        };
        let changes = match diff(&state.root, &state.original, &state.lines) {
            Ok(changes) => changes,
            Err(message) => {
                self.show_message(message);
                return false;
            }
        };
        if changes.is_empty() {
            self.editor.mark_current_clean();
            if visible {
                self.show_message("No directory changes".to_string());
            }
            self.complete_save(after);
            return true;
        }
        if directory_changed(state) {
            self.show_message("Directory changed on disk; reload before saving".to_string());
            return false;
        }
        let fs = RealFs::new(!self.directory_options.trash);
        let plan = match plan(changes.clone(), &fs) {
            Ok(plan) => plan,
            Err(message) => {
                self.show_message(message);
                return false;
            }
        };
        if !force && self.directory_options.confirm && plan.needs_confirmation() {
            let message = confirmation_message(&changes);
            self.show_message(message);
            self.directory.set_pending(PendingApply {
                number,
                changes,
                plan,
                after,
                visible,
            });
            return false;
        }
        self.begin_directory_apply(number, changes, plan, after, visible)
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

    pub(super) fn apply_pending_directory(&mut self, accept: bool) {
        let Some(pending) = self.directory.take_pending() else {
            return;
        };
        if !accept {
            self.show_message("Directory changes cancelled".to_string());
            return;
        }
        self.begin_directory_apply(
            pending.number,
            pending.changes,
            pending.plan,
            pending.after,
            pending.visible,
        );
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
        }
        self.refresh_after_directory_change();
        let reloaded = self.reload_directory(number);
        if let Some(error) = outcome.error {
            self.show_message(format!(
                "{}; applied {} operation(s)",
                error, outcome.applied
            ));
            return false;
        }
        if let Err(error) = reloaded {
            self.show_message(error);
            return false;
        }
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

fn confirmation_message(changes: &[Change]) -> String {
    let mut labels: Vec<String> = changes.iter().take(3).map(change_label).collect();
    if changes.len() > labels.len() {
        labels.push("...".to_string());
    }
    format!(
        "Apply {} change(s): {} [y/N]",
        changes.len(),
        labels.join(", ")
    )
}

fn change_label(change: &Change) -> String {
    match change {
        Change::Create { path, kind } => {
            let action = if *kind == EntryKind::Directory {
                "create directory"
            } else {
                "create"
            };
            format!("{action} {}", path.display())
        }
        Change::Rename { from, to, .. } => {
            format!("rename {} to {}", from.display(), to.display())
        }
        Change::Copy { from, to, .. } => {
            format!("copy {} to {}", from.display(), to.display())
        }
        Change::Delete { path, .. } => format!("delete {}", path.display()),
    }
}
