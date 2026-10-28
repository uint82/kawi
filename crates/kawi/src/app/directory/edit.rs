use kawi_editor::Action;

use super::super::file_ops::absolute_path;
use super::diff::parse_text;
use crate::app::App;

const NO_NAME: &str = "E32: No file name";

impl App {
    pub(in crate::app) fn sync_current_directory(&mut self) {
        let number = self.editor.buffer_number();
        let Some(state) = self.directory.get(number) else {
            return;
        };
        let revision = self.editor.buffer_revision();
        if state.revision == revision {
            return;
        }
        let texts = self.editor.text_lines();
        if let Some(state) = self.directory.get_mut(number) {
            state.sync(texts, revision);
        }
    }

    pub(super) fn open_directory_entry(&mut self) -> bool {
        let Some(number) = self.directory_buffer_number() else {
            return false;
        };
        let Some(line) = self
            .directory
            .get(number)
            .and_then(|state| state.lines.get(self.editor.row()))
            .cloned()
        else {
            return false;
        };
        let Some(state) = self.directory.get(number) else {
            return false;
        };
        let Ok((relative, is_directory)) = parse_text(&line.text) else {
            self.show_message("Invalid directory entry".to_string());
            return true;
        };
        let target = state.root.join(relative);
        if !target.exists() {
            self.show_message("Entry does not exist yet; save the listing first".to_string());
            return true;
        }
        if is_directory || target.is_dir() {
            self.open_directory_path(&target, true);
        } else if let Err(message) = self.open_file_location(&target, None, None) {
            self.show_message(message);
        }
        true
    }

    pub(in crate::app) fn open_directory_parent(&mut self) -> bool {
        if !self.editor.has_file() {
            self.show_message(NO_NAME.to_string());
            return true;
        }
        let current = absolute_path(self.editor.filename());
        let Some(parent) = current.parent() else {
            self.show_message("Already at the filesystem root".to_string());
            return true;
        };
        let Some(name) = current
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            return true;
        };
        let is_directory = self
            .directory
            .get(self.editor.buffer_number())
            .is_some_and(|state| state.root == current);
        let focus = if is_directory {
            format!("{name}/")
        } else {
            name
        };
        if self.open_directory_path(parent, true) {
            let number = self.editor.buffer_number();
            if self.directory_loading.is_loading(number) {
                self.directory_loading.defer_focus(number, focus);
            } else {
                self.focus_directory_line(number, &focus);
            }
        }
        true
    }

    pub(super) fn focus_directory_line(&mut self, number: usize, text: &str) -> bool {
        if number != self.editor.buffer_number() {
            return false;
        }
        let Some(row) = self
            .directory
            .get(number)
            .and_then(|state| state.lines.iter().position(|line| line.text == text))
        else {
            return false;
        };
        self.editor.apply(Action::JumpTo(row, 0));
        self.reveal_cursor();
        true
    }

    pub(super) fn toggle_directory_hidden(&mut self) -> bool {
        let number = self.editor.buffer_number();
        if self.directory_loading.is_loading(number) {
            self.show_message("Directory is still loading".to_string());
            return true;
        }
        if self.editor.buffer_is_dirty(number).unwrap_or(false) {
            self.show_message("Save or undo changes before toggling hidden files".to_string());
            return true;
        }
        let Some(state) = self.directory.get_mut(number) else {
            return false;
        };
        state.show_hidden = !state.show_hidden;
        let hidden = state.show_hidden;
        if let Err(message) = self.start_directory_reload(number) {
            if let Some(state) = self.directory.get_mut(number) {
                state.show_hidden = !state.show_hidden;
            }
            self.show_message(message);
            return true;
        }
        self.show_message(if hidden {
            "Showing hidden files".to_string()
        } else {
            "Hiding hidden files".to_string()
        });
        true
    }
}
