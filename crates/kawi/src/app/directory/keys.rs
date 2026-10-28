use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_editor::Mode;

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
        if self.directory_buffer_number().is_none() {
            return Ok(false);
        }
        match key.code {
            KeyCode::Char('-') => Ok(self.open_directory_parent()),
            KeyCode::Enter => Ok(self.open_directory_entry()),
            KeyCode::Char('.') => Ok(self.toggle_directory_hidden()),
            _ => Ok(false),
        }
    }
}
