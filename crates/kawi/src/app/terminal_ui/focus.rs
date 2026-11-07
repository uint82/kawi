use crossterm::event::KeyEvent;

use crate::app::App;

impl App {
    pub(crate) fn focus_terminal(&mut self) {
        if !self.terminal.is_open() {
            return;
        }
        self.cancel_pending_input();
        self.clear_focus();
        self.terminal_focus = true;
        if self.terminal.is_running() {
            self.terminal.focus_input();
        }
    }

    pub(crate) fn unfocus_terminal(&mut self) {
        self.terminal_focus = false;
        self.reveal_cursor();
    }

    pub(crate) fn toggle_terminal_focus(&mut self) -> anyhow::Result<bool> {
        if self.terminal_focus {
            self.unfocus_terminal();
            return Ok(false);
        }
        if self.terminal.is_open() {
            self.focus_terminal();
            return Ok(false);
        }
        self.open_terminal(None, self.terminal.layout())
    }

    pub(crate) fn handle_terminal_focus_key(
        &mut self,
        key: KeyEvent,
    ) -> anyhow::Result<Option<bool>> {
        if key != self.terminal_focus_key.event() {
            return Ok(None);
        }
        if !self.terminal_focus && self.is_transient_overlay_active() {
            return Ok(None);
        }
        self.toggle_terminal_focus().map(Some)
    }
}
