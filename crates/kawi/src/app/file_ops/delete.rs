use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use kawi_editor::paths;

use super::pending::{DeletePending, PendingFileOp};
use super::prompt::{DeleteRequest, DeleteSession, FileOpSession};
use super::trash::TrashDir;
use super::{absolute_path, buffer_numbers_under, fs as file_fs, with_warning, App};
use crate::app::apply_journal;
use crate::app::directory::fs::RealFs;
use crate::app::directory::temporary::{unique_path, TemporaryPath};
use crate::recovery::{ApplyJournal, JournalCleanup, JournalOperation, JournalPath};

impl App {
    pub(crate) fn open_delete_prompt(&mut self, path: PathBuf, permanent: bool) {
        let target = absolute_path(&path.to_string_lossy());
        let metadata = match fs::symlink_metadata(&target) {
            Ok(metadata) => metadata,
            Err(error) => {
                self.show_message(format!(
                    "E484: Can't open file {}: {error}",
                    target.display()
                ));
                return;
            }
        };
        if !delete_allowed(self, &target, permanent) {
            self.show_message(format!(
                "Refusing to delete protected path {} (use :delete! to override)",
                target.display()
            ));
            return;
        }
        let directory = metadata.is_dir() && !metadata.file_type().is_symlink();
        let items = directory
            .then(|| file_fs::count_tree(&target).ok())
            .flatten();
        let trash = TrashDir::for_target(&target);
        let can_trash = trash.as_ref().is_some_and(|trash| trash.can_trash(&target));
        let permanent_requested = permanent;
        let permanent = permanent || !can_trash;
        if !can_trash {
            self.show_message("Trash unavailable; type delete to permanently delete".to_string());
        }
        let dirty = buffer_numbers_under(self, &target)
            .into_iter()
            .filter(|number| self.editor.buffer_is_dirty(*number).unwrap_or(false))
            .collect();
        let request = DeleteRequest {
            target,
            permanent,
            dirty,
            items,
        };
        self.file_ops = Some(FileOpSession::Delete(DeleteSession::new(
            request,
            directory || (!permanent_requested && !can_trash),
        )));
    }

    pub(super) fn begin_delete(&mut self, request: DeleteRequest, keep_dirty: bool) {
        let request_id = if self.lsp.supports_file_delete() {
            self.lsp
                .will_delete_files(std::slice::from_ref(&request.target))
        } else {
            0
        };
        if request_id != 0 {
            let target = request.target.clone();
            self.pending_file_op = Some(PendingFileOp::Delete(DeletePending {
                request: request_id,
                delete: request,
                keep_dirty,
            }));
            self.show_message(format!("Deleting \"{}\"...", target.display()));
            return;
        }
        self.finish_delete(request, keep_dirty);
    }

    pub(super) fn finish_delete(&mut self, request: DeleteRequest, keep_dirty: bool) {
        let target = request.target.clone();
        let staged = match delete_stage_path(&target) {
            Ok(staged) => staged,
            Err(error) => {
                self.show_message(error);
                return;
            }
        };
        let warning = match self.delete_target(&request, &staged) {
            Ok(warning) => warning,
            Err(error) => {
                self.show_message(error);
                return;
            }
        };
        self.finish_deleted_state(&target, keep_dirty);
        self.show_message(delete_message(&request, &target, warning));
    }

    fn finish_deleted_state(&mut self, target: &Path, keep_dirty: bool) {
        self.lsp.did_delete_files(&[target.to_path_buf()]);
        let numbers = buffer_numbers_under(self, target);
        self.undo.remove_prefix(target);
        self.problems.remove_paths(target);
        self.explorer.remove_path(target);
        self.directory.remove_under(target);
        for number in numbers {
            if keep_dirty && self.editor.buffer_is_dirty(number).unwrap_or(false) {
                self.editor.detach_buffer_path(number);
            } else {
                let _ = self.editor.close_buffer(number, true);
            }
        }
        self.lsp.refresh(&self.editor);
        self.sync_language_settings();
        self.sync_watched_files();
        self.refresh_git_status();
    }

    fn delete_target(
        &self,
        request: &DeleteRequest,
        staged: &Path,
    ) -> Result<Option<String>, String> {
        let journal = self.create_delete_journal(&request.target, staged)?;
        let written = match journal.as_ref() {
            Some(journal) => apply_journal::persist(self.recovery.as_ref(), journal)?,
            None => false,
        };
        if let Err(error) = fs::rename(&request.target, staged) {
            let warning = apply_journal::finish_failure(self.recovery.as_ref(), true, written);
            return Err(with_warning(
                format!("Cannot delete {}: {error}", request.target.display()),
                warning,
            ));
        }
        let cleanup = delete_cleanup(request, staged);
        Ok(apply_journal::finish_success(
            self.recovery.as_ref(),
            journal.as_ref(),
            &[cleanup],
            written,
        ))
    }

    fn create_delete_journal(
        &self,
        target: &Path,
        staged: &Path,
    ) -> Result<Option<ApplyJournal>, String> {
        if self.recovery.is_none() {
            return Ok(None);
        }
        Ok(Some(ApplyJournal::prepared(vec![
            JournalOperation::Delete {
                path: JournalPath::from(target.to_path_buf()),
                staged: JournalPath::from(staged.to_path_buf()),
            },
        ])))
    }
}

fn delete_message(request: &DeleteRequest, target: &Path, warning: Option<String>) -> String {
    let action = match (request.permanent, warning.is_some()) {
        (_, true) => "Removed",
        (true, false) => "Deleted",
        (false, false) => "Trashed",
    };
    with_warning(format!("{action} \"{}\"", target.display()), warning)
}

fn delete_stage_path(target: &Path) -> Result<PathBuf, String> {
    let parent = target.parent().unwrap_or_else(|| Path::new(""));
    let spec = TemporaryPath::new(target, parent, "kawi-delete", 0);
    unique_path(&spec, &BTreeSet::new(), &RealFs::new(true))
        .map_err(|error| format!("Cannot prepare delete for {}: {error}", target.display()))
}

fn delete_cleanup(request: &DeleteRequest, staged: &Path) -> JournalCleanup {
    if request.permanent {
        JournalCleanup::remove(staged.to_path_buf())
    } else {
        JournalCleanup::trash(staged.to_path_buf(), request.target.clone())
    }
}

fn delete_allowed(app: &App, target: &Path, force: bool) -> bool {
    if force {
        return true;
    }
    let target = canonical_or_absolute(target);
    let workspace = canonical_or_absolute(&app.picker_root());
    let home = kawi_editor::paths::home_dir().map(|path| canonical_or_absolute(&path));
    let cwd = std::env::current_dir()
        .ok()
        .map(|path| canonical_or_absolute(&path));
    !is_protected(&target, &workspace)
        && !home.is_some_and(|home| is_protected(&target, &home))
        && !cwd.is_some_and(|cwd| is_protected(&target, &cwd))
}

fn is_protected(target: &Path, protected: &Path) -> bool {
    protected.starts_with(target)
}

fn canonical_or_absolute(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| {
        paths::absolute(
            &path.to_string_lossy(),
            &std::env::current_dir().unwrap_or_default(),
        )
    })
}
