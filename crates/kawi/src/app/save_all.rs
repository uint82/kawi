use anyhow::Result;

use super::save::SaveAfter;
use super::App;

impl App {
    pub(super) fn handle_save_all(&mut self) -> Result<bool> {
        self.save_all(SaveAfter::None)
    }

    pub(super) fn handle_write_quit_all(&mut self) -> Result<bool> {
        self.save_all(SaveAfter::QuitAll)
    }

    pub(super) fn save_is_pending(&self) -> bool {
        self.pending_save.is_some()
            || self.pending_save_all.is_some()
            || self.directory.has_pending()
            || self.pending_file_op.is_some()
    }

    pub(super) fn finish_save_all_files(&mut self, after: SaveAfter) -> bool {
        match self.editor.write_all() {
            Ok(()) => {
                self.persist_clean_undos();
                self.record_all_saved();
                self.lsp.refresh(&self.editor);
                self.lsp.save();
                if after == SaveAfter::QuitAll {
                    self.quit_pending = true;
                }
                true
            }
            Err(error) => {
                self.show_message(super::file_error_message(error));
                false
            }
        }
    }

    fn save_all(&mut self, after: SaveAfter) -> Result<bool> {
        if self.save_is_pending() {
            self.show_message("Save already in progress".to_string());
            return Ok(false);
        }
        if self.has_disk_conflicts() {
            self.show_message("Resolve disk conflicts before saving all".to_string());
            return Ok(false);
        }
        let saved = if self.has_dirty_directory() {
            self.start_directory_save_all(after)
        } else {
            self.finish_save_all_files(after)
        };
        Ok(saved && after == SaveAfter::QuitAll)
    }
}
