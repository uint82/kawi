use crossterm::event::KeyEvent;

use crate::app::App;

impl App {
    pub(crate) fn focus_terminal(&mut self) {
        if !self.terminal.is_open() {
            return;
        }
        self.terminal_focus = true;
        if self.terminal.is_running() {
            self.terminal.focus_input();
        }
    }

    pub(crate) fn unfocus_terminal(&mut self) {
        self.terminal_focus = false;
        self.reveal_cursor();
    }

    pub(crate) fn toggle_terminal_focus(&mut self) {
        if self.terminal_focus {
            self.unfocus_terminal();
        } else if self.terminal.is_open() {
            self.focus_terminal();
        }
    }

    pub(crate) fn handle_terminal_focus_key(&mut self, key: KeyEvent) -> Option<bool> {
        if key != self.terminal_focus_key.event() {
            return None;
        }
        self.toggle_terminal_focus();
        Some(false)
    }
}
