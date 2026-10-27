use std::fs;
use std::path::Path;

use super::pending::{PendingFileOp, RenamePending};
use super::{absolute_path, App};

impl App {
    pub(super) fn rename_file(&mut self, source: &Path, target: &str, force: bool) {
        let old = absolute_path(&source.to_string_lossy());
        let new = absolute_path(target);
        if old == new {
            self.show_message("Old and new path are the same".to_string());
            return;
        }
        let metadata = match fs::symlink_metadata(&old) {
            Ok(metadata) => metadata,
            Err(error) => {
                self.show_message(format!("E484: Can't open file {}: {error}", old.display()));
                return;
            }
        };
        if let Some(parent) = super::fs::missing_parent(&new) {
            self.show_message(format!(
                "E484: Can't open file {}: parent directory {} does not exist",
                new.display(),
                parent.display()
            ));
            return;
        }
        let directory = metadata.is_dir() && !metadata.file_type().is_symlink();
        let request = if self.lsp.supports_file_rename() {
            self.lsp.will_rename_files(&[(old.clone(), new.clone())])
        } else {
            0
        };
        if request != 0 {
            self.pending_file_op = Some(PendingFileOp::Rename(RenamePending {
                request,
                source: old.clone(),
                target: new,
                force,
                directory,
            }));
            self.show_message(format!("Renaming \"{}\"...", old.display()));
            return;
        }
        self.finish_rename(old, new, force, directory);
    }

    pub(super) fn finish_rename(
        &mut self,
        old: std::path::PathBuf,
        new: std::path::PathBuf,
        force: bool,
        directory: bool,
    ) {
        let outcome = match super::fs::rename_path(&old, &new, force) {
            Ok(outcome) => outcome,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                self.show_message("E13: File exists (add ! to override)".to_string());
                return;
            }
            Err(error) => {
                self.show_message(format!(
                    "Cannot rename {} to {}: {error}",
                    old.display(),
                    new.display()
                ));
                return;
            }
        };
        self.lsp.did_rename_files(&[(old.clone(), new.clone())]);
        let adoptions = self.editor.rewrite_buffer_paths(&old, &new);
        self.directory.rewrite_path(&old, &new);
        self.undo.move_prefix(&old, &new);
        self.problems.rewrite_paths(&old, &new);
        self.explorer.rename_path(&old, &new);
        for adoption in &adoptions {
            self.record_saved_buffer_number(adoption.number);
        }
        self.lsp.refresh(&self.editor);
        self.sync_language_settings();
        self.sync_watched_files();
        self.refresh_git_status();
        let mut note = String::new();
        if outcome.symlink {
            note.push_str(" (symlink moved, target unchanged)");
        }
        if outcome.kind == super::fs::RenameKind::Copied {
            note.push_str(" (copied and deleted source across filesystems)");
        }
        let kind = if directory { "directory" } else { "file" };
        self.show_message(format!(
            "Renamed {kind} \"{}\" to \"{}\"{note}",
            old.display(),
            new.display()
        ));
    }
}
