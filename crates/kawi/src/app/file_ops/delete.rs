use std::fs;
use std::path::{Path, PathBuf};

use kawi_editor::paths;

use super::pending::{DeletePending, PendingFileOp};
use super::prompt::{DeleteRequest, DeleteSession, FileOpSession};
use super::trash::TrashDir;
use super::{absolute_path, buffer_numbers_under, fs as file_fs, App};

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
        let target = &request.target;
        let result = if request.permanent {
            file_fs::remove_path(target, true)
        } else {
            match TrashDir::for_target(target) {
                Some(trash) => trash
                    .trash(target)
                    .map(|_| ())
                    .map_err(|error| match error {
                        super::trash::TrashError::Io(error) => error,
                        super::trash::TrashError::Unavailable(message) => {
                            std::io::Error::other(message)
                        }
                    }),
                None => Err(std::io::Error::other("trash is unavailable")),
            }
        };
        if let Err(error) = result {
            self.show_message(format!("Cannot delete {}: {error}", target.display()));
            return;
        }
        self.lsp.did_delete_files(std::slice::from_ref(target));
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
        let action = if request.permanent {
            "Deleted"
        } else {
            "Trashed"
        };
        self.show_message(format!("{action} \"{}\"", target.display()));
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
