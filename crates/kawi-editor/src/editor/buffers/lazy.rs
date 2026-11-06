use std::path::Path;

use kawi_buffer::Buffer;

use super::super::options::LanguageIndent;
use super::super::Editor;
use super::{current_dir, ParkedState};
use crate::cursor::Cursor;
use crate::editing::LanguageEditing;
use crate::folds::FoldState;
use crate::paths;

impl Editor {
    pub fn add_file_placeholder(&mut self, name: &str) -> usize {
        let target = paths::absolute(name, &current_dir());
        if let Some(number) = self.find_buffer_by_path(&target) {
            self.buffers.set_listed(number, true);
            return number;
        }
        let mut buffer = Buffer::from_string("");
        buffer.path = Some(name.to_string());
        buffer.mark_clean();
        let state = self.parked_state(buffer);
        let number = self.buffers.allocate_number();
        self.buffers.insert(number, state);
        number
    }

    pub fn replace_parked_buffer(&mut self, number: usize, buffer: Buffer) -> bool {
        let Some(entry) = self
            .buffers
            .parked
            .iter_mut()
            .find(|entry| entry.number == number)
        else {
            return false;
        };
        if !entry.state.pending {
            return false;
        }
        entry.state.install(buffer);
        true
    }

    pub fn is_buffer_pending(&self, number: usize) -> bool {
        self.buffers
            .entry(number)
            .is_some_and(|entry| entry.state.pending)
    }

    pub(super) fn load_pending_parked(&mut self, number: usize) {
        let Some(entry) = self
            .buffers
            .parked
            .iter_mut()
            .find(|entry| entry.number == number)
        else {
            return;
        };
        if !entry.state.pending {
            return;
        }
        let Some(path) = entry.state.buffer.path.clone() else {
            entry.state.pending = false;
            return;
        };
        match Buffer::open(&path) {
            Ok(buffer) => entry.state.install(buffer),
            Err(_) => entry.state.pending = false,
        }
    }

    fn find_buffer_by_path(&self, target: &Path) -> Option<usize> {
        let cwd = current_dir();
        if self
            .buffer
            .path
            .as_deref()
            .is_some_and(|path| absolute_path_matches(path, target, &cwd))
        {
            return Some(self.buffers.current());
        }
        self.buffers.parked.iter().find_map(|entry| {
            let path = entry.state.buffer.path.as_deref()?;
            absolute_path_matches(path, target, &cwd).then_some(entry.number)
        })
    }

    fn parked_state(&self, buffer: Buffer) -> ParkedState {
        let mut folds = FoldState::default();
        folds.set_method(self.fold_method);
        folds.set_column(self.fold_column);
        folds.refresh(&buffer);
        ParkedState {
            buffer,
            listed: true,
            cursor: Cursor::default(),
            runtime_shiftwidth: None,
            runtime_expandtab: None,
            language_indent: LanguageIndent::default(),
            language_editing: LanguageEditing::default(),
            autoindent: self.buffer_defaults.autoindent,
            viewport_top: 0,
            folds,
            last_change: None,
            undone_change: None,
            insert_repeat: None,
            insert_reopen: false,
            insert_record: Vec::new(),
            insert_moved: false,
            insert_last_char: None,
            search_status: None,
            change_list: Vec::new(),
            change_index: 0,
            pending: true,
        }
    }
}

fn absolute_path_matches(path: &str, target: &Path, cwd: &Path) -> bool {
    let path = Path::new(path);
    if path.is_absolute() {
        return path == target;
    }
    paths::absolute(&path.to_string_lossy(), cwd) == target
}

impl ParkedState {
    fn install(&mut self, mut buffer: Buffer) {
        buffer.set_id(self.buffer.id().to_string());
        self.buffer = buffer;
        self.pending = false;
    }
}

#[cfg(test)]
mod tests;
