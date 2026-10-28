use std::path::Path;

use super::super::file_ops::absolute_path;
use super::listing::Listing;
use super::session::DirectoryBuffer;
use crate::app::App;

impl App {
    pub(crate) fn open_directory_path(&mut self, path: &Path, reuse_empty: bool) -> bool {
        match self.open_directory(path, reuse_empty) {
            Ok(_) => true,
            Err(message) => {
                self.show_message(message);
                false
            }
        }
    }

    pub(crate) fn open_directory(
        &mut self,
        path: &Path,
        reuse_empty: bool,
    ) -> Result<usize, String> {
        let root = absolute_path(&path.to_string_lossy());
        if !root.is_dir() {
            return Err(format!("E484: Not a directory: {}", root.display()));
        }
        if let Some(number) = self.directory.find_root(&root) {
            if !self.editor.buffer_is_dirty(number).unwrap_or(false)
                && !self.directory_loading.is_loading(number)
            {
                self.start_directory_reload(number)?;
            }
            self.editor.buffer_select(number);
            return Ok(number);
        }
        let options = self.directory_options;
        let number = self
            .editor
            .open_directory_buffer(&root.to_string_lossy(), "", reuse_empty);
        let revision = self.editor.buffer_revision_at(number).unwrap_or_default();
        let state = DirectoryBuffer::new(
            root,
            Listing::empty(),
            options.show_hidden,
            options.sort,
            revision,
        );
        self.directory.insert(number, state);
        if let Err(message) = self.start_directory_initial_load(number) {
            self.directory.remove(number);
            let _ = self.editor.close_buffer(number, true);
            return Err(message);
        }
        self.after_directory_open();
        Ok(number)
    }

    pub(in crate::app) fn directory_buffer_number(&self) -> Option<usize> {
        let number = self.editor.buffer_number();
        self.directory.contains(number).then_some(number)
    }

    pub(in crate::app) fn has_dirty_directory(&self) -> bool {
        self.directory
            .numbers()
            .any(|number| self.editor.buffer_is_dirty(number).unwrap_or(false))
    }

    fn after_directory_open(&mut self) {
        self.sync_language_settings();
        self.sync_watched_files();
        self.refresh_git_status();
        self.reveal_cursor();
    }
}
