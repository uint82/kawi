use std::path::Path;

use kawi_editor::PathAdoption;

use super::App;

impl App {
    pub(in crate::app) fn move_paths(&mut self, old: &Path, new: &Path) -> Vec<PathAdoption> {
        let adoptions = self.editor.rewrite_buffer_paths(old, new);
        self.directory.rewrite_path(old, new);
        self.undo.move_prefix(old, new);
        self.problems.rewrite_paths(old, new);
        self.explorer.rename_path(old, new);
        adoptions
    }

    pub(in crate::app) fn rewrite_paths(&mut self, old: &Path, new: &Path) {
        let adoptions = self.move_paths(old, new);
        for adoption in &adoptions {
            self.record_saved_buffer_number(adoption.number);
        }
    }

    pub(in crate::app) fn notify_path_move(&mut self, old: &Path, new: &Path) {
        self.lsp
            .did_rename_files(&[(old.to_path_buf(), new.to_path_buf())]);
        self.lsp.refresh(&self.editor);
        self.sync_language_settings();
        self.sync_watched_files();
        self.refresh_git_status();
    }
}
