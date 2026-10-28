use std::path::Path;

use super::super::file_ops::absolute_path;
use super::listing::{read_listing, Listing};
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
            if !self.editor.buffer_is_dirty(number).unwrap_or(false) {
                self.reload_directory(number)?;
            }
            self.editor.buffer_select(number);
            return Ok(number);
        }
        let options = self.directory_options;
        let listing = read_listing(&root, options.show_hidden, 1, options.sort)
            .map_err(|error| format!("Cannot read {}: {error}", root.display()))?;
        let text = listing_text(&listing);
        let number = self
            .editor
            .open_directory_buffer(&root.to_string_lossy(), &text, reuse_empty);
        let revision = self.editor.buffer_revision_at(number).unwrap_or_default();
        let state =
            DirectoryBuffer::new(root, listing, options.show_hidden, options.sort, revision);
        self.directory.insert(number, state);
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

    pub(in crate::app) fn reload_directory(&mut self, number: usize) -> Result<(), String> {
        let (root, show_hidden, sort) = {
            let state = self
                .directory
                .get(number)
                .ok_or_else(|| "directory buffer is missing".to_string())?;
            (state.root.clone(), state.show_hidden, state.sort)
        };
        let listing = read_listing(&root, show_hidden, 1, sort)
            .map_err(|error| format!("Cannot read {}: {error}", root.display()))?;
        let text = listing_text(&listing);
        self.editor
            .replace_directory_listing(number, &root.to_string_lossy(), &text);
        let revision = self.editor.buffer_revision_at(number).unwrap_or_default();
        if let Some(state) = self.directory.get_mut(number) {
            state.reset_listing(listing, revision);
        }
        if number == self.editor.buffer_number() {
            self.reveal_cursor();
        }
        Ok(())
    }

    fn after_directory_open(&mut self) {
        self.sync_language_settings();
        self.sync_watched_files();
        self.refresh_git_status();
        self.reveal_cursor();
    }
}

fn listing_text(listing: &Listing) -> String {
    listing
        .entries
        .iter()
        .map(|entry| entry.line_text())
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn directory_changed(state: &DirectoryBuffer) -> bool {
    let Ok(listing) = read_listing(&state.root, state.show_hidden, 1, state.sort) else {
        return true;
    };
    state.original.len() != listing.entries.len()
        || state
            .original
            .iter()
            .zip(&listing.entries)
            .any(|(old, new)| old.name != new.name || old.kind != new.kind)
}
