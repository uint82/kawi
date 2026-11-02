use super::super::save::SaveAfter;
use super::diff::{diff, Change};
use super::executor::{Outcome, PreparedPlan};
use super::fs::RealFs;
use super::load::DIRECTORY_LOADING_MESSAGE;
use super::permission_check::PermissionPhase;
use super::session::{DirectoryEdit, PendingApply};
use super::verify::DELETED_SAVE_MESSAGE;
use crate::app::apply_journal;
use crate::app::App;
use changes::rename_requests;
use messages::{apply_success_message, failure_message};

mod changes;
mod messages;

pub(super) use changes::all_changes;

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
        if self.reject_deleted_directory_save(number, visible) {
            return true;
        }
        if self.reject_running_permission_check(visible) {
            return true;
        }
        if !self.directory_loading.is_loading(number) {
            return false;
        }
        if visible {
            self.show_message(DIRECTORY_LOADING_MESSAGE.to_string());
        }
        true
    }

    pub(super) fn reject_deleted_directory_save(&mut self, number: usize, visible: bool) -> bool {
        if !self.directory.deleted(number) {
            return false;
        }
        if visible {
            self.show_message(DELETED_SAVE_MESSAGE.to_string());
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
        let prepared = match PreparedPlan::new(&apply.plan, &fs) {
            Ok(prepared) => prepared,
            Err(error) => return self.cancel_prepared_apply(&apply, error),
        };
        let journal = match self.create_apply_journal(&prepared) {
            Ok(journal) => journal,
            Err(error) => return self.cancel_prepared_apply(&apply, error),
        };
        let journal_written = match journal.as_ref() {
            Some(journal) => match apply_journal::persist(self.recovery.as_ref(), journal) {
                Ok(written) => written,
                Err(error) => return self.cancel_prepared_apply(&apply, error),
            },
            None => false,
        };
        let promotions = self.recovery.as_ref().map(|manager| manager.store_clone());
        let mut record_promotion = |operation| match &promotions {
            Some(store) => store
                .append_apply_promotion(operation)
                .map_err(std::io::Error::other),
            None => Ok(()),
        };
        let outcome = prepared.execute(&mut fs, &mut record_promotion);
        for index in &outcome.remaining {
            if let Some(operation) = apply.plan.operations.get(*index) {
                self.update_applied_operation(operation);
            }
        }
        if outcome.error.is_none() {
            self.notify_lsp_changes(&changes);
            for edit in &apply.edits {
                self.editor.mark_buffer_clean(edit.number);
            }
        }
        self.refresh_after_directory_change();
        match outcome.error {
            Some(_) => {
                let warning = apply_journal::finish_failure(
                    self.recovery.as_ref(),
                    outcome.rollback_errors.is_empty(),
                    journal_written,
                );
                self.finish_directory_apply_failure(&apply, &outcome, warning)
            }
            None => {
                let cleanups = outcome
                    .deferred_deletes
                    .iter()
                    .cloned()
                    .map(crate::recovery::JournalCleanup::remove)
                    .collect::<Vec<_>>();
                let warning = apply_journal::finish_success(
                    self.recovery.as_ref(),
                    journal.as_ref(),
                    &cleanups,
                    journal_written,
                );
                let saved = self.finish_directory_apply_success(&apply);
                if let Some(warning) = warning {
                    self.show_message(warning);
                }
                saved
            }
        }
    }

    fn cancel_prepared_apply(&mut self, apply: &PendingApply, error: String) -> bool {
        self.cancel_directory_apply(&apply.edits);
        self.show_message(error);
        false
    }

    fn create_apply_journal(
        &self,
        prepared: &PreparedPlan<'_>,
    ) -> Result<Option<crate::recovery::ApplyJournal>, String> {
        if self.recovery.is_none() {
            return Ok(None);
        }
        apply_journal::create_for_plan(prepared).map(Some)
    }

    pub(super) fn finish_directory_apply_failure(
        &mut self,
        apply: &PendingApply,
        outcome: &Outcome,
        warning: Option<String>,
    ) -> bool {
        let reload_error = self.reload_directories(&apply.edits);
        let mut message = failure_message(&apply.plan, outcome);
        if let Some(reload_error) = reload_error {
            message.push_str("; ");
            message.push_str(&reload_error);
        }
        if let Some(warning) = warning {
            message.push_str("; ");
            message.push_str(&warning);
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
            Operation::Rename { from, to, .. } => {
                self.rewrite_paths(from, to);
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

    pub(super) fn refresh_after_directory_change(&mut self) {
        self.lsp.refresh(&self.editor);
        self.sync_language_settings();
        self.sync_watched_files();
        self.refresh_git_status();
        self.explorer.refresh();
    }
}
