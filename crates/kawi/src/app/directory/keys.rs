use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_editor::Mode;

use super::load::DIRECTORY_LOADING_MESSAGE;
use crate::app::App;
use crate::keymap::Keymap;

impl App {
    pub(in crate::app) fn handle_directory_confirm_key(
        &mut self,
        key: KeyEvent,
    ) -> anyhow::Result<bool> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => self.apply_pending_directory(true),
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                self.apply_pending_directory(false);
            }
            _ => {}
        }
        Ok(false)
    }

    pub(in crate::app) fn handle_directory_key(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        if self.editor.mode() != Mode::Normal
            || self.keymap.pending_text().is_some()
            || key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
        {
            return Ok(false);
        }
        let Some(number) = self.directory_buffer_number() else {
            return Ok(false);
        };
        if key.code == KeyCode::Char('-') {
            return Ok(self.open_directory_parent());
        }
        if self.directory_loading.is_loading(number) {
            if matches!(key.code, KeyCode::Enter | KeyCode::Char('.')) {
                self.show_message(DIRECTORY_LOADING_MESSAGE.to_string());
                return Ok(true);
            }
            return Ok(false);
        }
        match key.code {
            KeyCode::Enter => Ok(self.open_directory_entry()),
            KeyCode::Char('.') => Ok(self.toggle_directory_hidden()),
            _ => Ok(false),
        }
    }
}
