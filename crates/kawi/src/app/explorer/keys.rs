use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_editor::command::ExplorerCommand;
use kawi_editor::Action;

use super::super::App;
use super::ExplorerRowKind;
use crate::keymap::Keymap;

impl App {
    pub(crate) fn toggle_explorer(&mut self) {
        if self.explorer.is_visible() {
            self.explorer.close();
            self.sync_terminal_panel_size();
            return;
        }
        self.open_explorer_at(self.explorer_root());
    }

    pub(crate) fn focus_explorer(&mut self) {
        if !self.explorer.is_visible() {
            self.open_explorer_at(self.explorer_root());
            return;
        }
        self.cancel_pending_editor_input();
        self.explorer.focus();
    }

    pub(crate) fn reveal_current_in_explorer(&mut self) {
        if !self.editor.has_file() {
            self.focus_explorer();
            return;
        }
        let root = self.explorer_root();
        if self.explorer.root() != Some(root.as_path()) {
            self.open_explorer_at(root);
            return;
        }
        self.cancel_pending_editor_input();
        self.explorer.focus();
        self.reveal_editor_file();
    }

    pub(crate) fn open_explorer_at(&mut self, root: PathBuf) {
        self.cancel_pending_editor_input();
        self.explorer.open(root);
        self.sync_terminal_panel_size();
        self.reveal_editor_file();
    }

    pub(crate) fn poll_explorer(&mut self) -> bool {
        self.explorer.poll_watch()
    }

    pub(crate) fn handle_explorer_command(
        &mut self,
        command: ExplorerCommand,
    ) -> anyhow::Result<bool> {
        match command {
            ExplorerCommand::Toggle => self.toggle_explorer(),
            ExplorerCommand::Focus => self.focus_explorer(),
            ExplorerCommand::Reveal => self.reveal_current_in_explorer(),
        }
        Ok(false)
    }

    pub(crate) fn handle_explorer_key(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        if is_explorer_cancel_key(key) {
            self.explorer.focus_editor();
            return Ok(false);
        }
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return Ok(false);
        }
        let height = self.explorer_content_height().max(1);
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.explorer.move_selection(1, height),
            KeyCode::Up | KeyCode::Char('k') => self.explorer.move_selection(-1, height),
            KeyCode::Home | KeyCode::Char('g') => self.explorer.move_first(height),
            KeyCode::End | KeyCode::Char('G') => self.explorer.move_last(height),
            KeyCode::PageDown => self.explorer.move_selection(height as isize, height),
            KeyCode::PageUp => self.explorer.move_selection(-(height as isize), height),
            KeyCode::Left | KeyCode::Char('h') => self.collapse_explorer_selected(height),
            KeyCode::Right | KeyCode::Char('l') => {
                self.expand_explorer_selected(height)?;
            }
            KeyCode::Enter => {
                self.open_explorer_selected(None)?;
            }
            KeyCode::Char('o') => {
                self.open_explorer_selected(None)?;
            }
            KeyCode::Char('v') => {
                self.open_explorer_selected(Some(true))?;
            }
            KeyCode::Char('s') => {
                self.open_explorer_selected(Some(false))?;
            }
            KeyCode::Char('a') => self.new_explorer_file(),
            KeyCode::Char('A') => self.new_explorer_directory(),
            KeyCode::Char('r') => self.rename_explorer_selected(),
            KeyCode::Char('d') => self.delete_explorer_selected(),
            KeyCode::Char('R') => self.explorer.refresh_and_watch(),
            KeyCode::Char('.') => self.toggle_explorer_hidden(),
            _ => {}
        }
        Ok(false)
    }

    pub(super) fn open_explorer_selected(&mut self, split: Option<bool>) -> anyhow::Result<bool> {
        let Some(row) = self.explorer.selected_row().cloned() else {
            return Ok(false);
        };
        if matches!(row.kind, ExplorerRowKind::Directory { .. }) {
            self.explorer.toggle_directory(&row.path);
            return Ok(false);
        }
        self.open_explorer_file(&row.path, split)
    }

    fn expand_explorer_selected(&mut self, height: usize) -> anyhow::Result<bool> {
        let Some(row) = self.explorer.selected_row().cloned() else {
            return Ok(false);
        };
        match row.kind {
            ExplorerRowKind::Directory { expanded: false } => {
                self.explorer.expand_directory(&row.path);
            }
            ExplorerRowKind::Directory { expanded: true } => {
                self.select_explorer_child(height);
            }
            _ => return self.open_explorer_file(&row.path, None),
        }
        Ok(false)
    }

    fn select_explorer_child(&mut self, height: usize) {
        let Some(index) = self.explorer.selected_index() else {
            return;
        };
        let next = index.saturating_add(1);
        let Some(row) = self.explorer.rows().get(next) else {
            return;
        };
        if row.depth <= self.explorer.rows()[index].depth {
            return;
        }
        let path = row.path.clone();
        self.explorer.select(path);
        self.explorer.ensure_visible(height);
    }

    fn collapse_explorer_selected(&mut self, height: usize) {
        let Some(row) = self.explorer.selected_row().cloned() else {
            return;
        };
        if matches!(row.kind, ExplorerRowKind::Directory { expanded: true }) {
            self.explorer.collapse_directory(&row.path);
            return;
        }
        let Some(parent) = row.path.parent() else {
            return;
        };
        if self
            .explorer
            .rows()
            .iter()
            .any(|entry| entry.path == parent)
        {
            self.explorer.select(parent.to_path_buf());
            self.explorer.ensure_visible(height);
        }
    }

    fn open_explorer_file(&mut self, path: &Path, split: Option<bool>) -> anyhow::Result<bool> {
        if let Some(vertical) = split {
            return self.open_explorer_split(path, vertical);
        }
        match self.open_file_location(path, None, None) {
            Ok(()) => {
                self.explorer.focus_editor();
                self.reveal_cursor();
                Ok(false)
            }
            Err(message) => {
                self.show_message(message);
                Ok(false)
            }
        }
    }

    fn open_explorer_split(&mut self, path: &Path, vertical: bool) -> anyhow::Result<bool> {
        match self.split_window(vertical, Some(path.to_string_lossy().into_owned())) {
            Ok(result) => {
                self.explorer.focus_editor();
                self.reveal_cursor();
                Ok(result)
            }
            Err(error) => {
                self.show_message(super::super::compact_error(&error.to_string()));
                Ok(false)
            }
        }
    }

    fn new_explorer_file(&mut self) {
        let directory = self.explorer_target_directory();
        self.open_new_file_prompt(directory);
    }

    fn new_explorer_directory(&mut self) {
        let directory = self.explorer_target_directory();
        self.open_new_directory_prompt(directory);
    }

    fn rename_explorer_selected(&mut self) {
        let Some(path) = self.explorer.selected_row().map(|row| row.path.clone()) else {
            return;
        };
        self.open_rename_prompt(path, false);
    }

    fn delete_explorer_selected(&mut self) {
        let Some(path) = self.explorer.selected_row().map(|row| row.path.clone()) else {
            return;
        };
        self.open_delete_prompt(path, false);
    }

    fn toggle_explorer_hidden(&mut self) {
        self.explorer.set_show_hidden(!self.explorer.show_hidden());
    }

    fn reveal_editor_file(&mut self) {
        if !self.editor.has_file() {
            return;
        }
        let path = kawi_editor::paths::absolute(
            self.editor.filename(),
            &std::env::current_dir().unwrap_or_default(),
        );
        self.explorer.reveal_path(&path);
        self.explorer.ensure_visible(self.explorer_content_height());
    }

    fn explorer_target_directory(&self) -> PathBuf {
        if let Some(row) = self.explorer.selected_row() {
            if matches!(row.kind, ExplorerRowKind::Directory { .. }) {
                return row.path.clone();
            }
            if let Some(parent) = row.path.parent() {
                return parent.to_path_buf();
            }
        }
        self.explorer
            .root()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.explorer_root())
    }

    fn cancel_pending_editor_input(&mut self) {
        self.keymap.reset_pending();
        self.editor.apply(Action::CancelPending);
    }
}

fn is_explorer_cancel_key(key: KeyEvent) -> bool {
    key.code == KeyCode::Esc
        || key.code == KeyCode::Char('q')
        || (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c'))
}
