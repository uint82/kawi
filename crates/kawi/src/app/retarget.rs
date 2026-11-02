use std::path::Path;

use super::App;

impl App {
    pub(in crate::app) fn rewrite_paths(&mut self, old: &Path, new: &Path) {
        let adoptions = self.editor.rewrite_buffer_paths(old, new);
        self.directory.rewrite_path(old, new);
        self.undo.move_prefix(old, new);
        self.problems.rewrite_paths(old, new);
        self.explorer.rename_path(old, new);
        for adoption in &adoptions {
            self.record_saved_buffer_number(adoption.number);
        }
    }
}
