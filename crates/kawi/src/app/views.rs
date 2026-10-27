use std::path::Path;

use anyhow::Result;

use super::App;
use crate::file_views::FileViewStore;
use crate::fold_view::FoldView;

impl App {
    pub(crate) fn attach_file_views(&mut self, store: FileViewStore) {
        self.file_views = Some(store);
    }

    pub(crate) fn load_file_views(&mut self) {
        let numbers = self.editor.all_buffer_numbers();
        let mut loaded = false;
        for number in numbers {
            if !self.directory.contains(number) {
                loaded |= self.load_file_view(number);
            }
        }
        if loaded {
            self.reveal_cursor();
        }
    }

    pub(crate) fn sync_file_view(&mut self) {
        let number = self.editor.buffer_number();
        if self.directory.contains(number) {
            return;
        }
        if self.load_file_view(number) {
            self.reveal_cursor();
        }
    }

    pub(crate) fn save_file_views(&mut self) -> Result<()> {
        let numbers = self.editor.all_buffer_numbers();
        let mut first_error = None;
        for number in numbers {
            if self.directory.contains(number) {
                continue;
            }
            if let Err(error) = self.save_file_view(number) {
                first_error.get_or_insert(error);
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    fn load_file_view(&mut self, number: usize) -> bool {
        let Some(store) = self.file_views.clone() else {
            return false;
        };
        let Some((path, id)) = self.buffer_view_key(number) else {
            return false;
        };
        if !self.loaded_views.insert((path.clone(), id)) {
            return false;
        }
        let Some(view) = store.load(Path::new(&path)) else {
            return false;
        };
        let Some(content_hash) = self.editor.buffer_content_hash(number) else {
            return false;
        };
        let snapshot = view.restore_snapshot(content_hash, self.editor.fold_method());
        if !self.editor.restore_buffer_folds(number, snapshot) {
            return false;
        }
        self.editor.refresh_buffer_fold_state(number);
        true
    }

    fn save_file_view(&mut self, number: usize) -> Result<()> {
        let Some(store) = self.file_views.clone() else {
            return Ok(());
        };
        let Some((path, _)) = self.buffer_view_key(number) else {
            return Ok(());
        };
        match self.fold_view(number) {
            Some(folds) => store.save(Path::new(&path), &folds),
            None => store.remove(Path::new(&path)),
        }
    }

    fn fold_view(&self, number: usize) -> Option<FoldView> {
        let snapshot = self.editor.buffer_fold_snapshot(number)?;
        if snapshot.is_empty() {
            return None;
        }
        let content_hash = self.editor.buffer_content_hash(number)?;
        Some(FoldView::from_snapshot(
            snapshot,
            self.editor.fold_method(),
            content_hash,
        ))
    }

    fn buffer_view_key(&self, number: usize) -> Option<(String, String)> {
        let path = self.editor.buffer_path(number)?.to_string();
        let id = self.editor.buffer_id(number)?.to_string();
        Some((path, id))
    }
}

#[cfg(test)]
mod tests;
