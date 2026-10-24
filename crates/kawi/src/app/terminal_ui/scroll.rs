use kawi_editor::clipboard::{Clipboard, ClipboardKind};
use kawi_tui::clipboard::SystemClipboard;

use super::{TerminalMode, TerminalUi};

impl TerminalUi {
    pub fn scroll_wheel(&mut self, delta: isize) -> bool {
        self.scroll_by(-delta)
    }

    fn scroll_by(&mut self, delta: isize) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        let screen = session.screen_mut();
        let current = screen.scrollback() as isize;
        let next = (current + delta).clamp(0, screen.scrollback_len() as isize);
        if next == current {
            return false;
        }
        screen.set_scrollback(next as usize);
        true
    }

    pub(super) fn scroll_up(&mut self, amount: usize) -> bool {
        self.scroll_by(amount as isize)
    }

    pub(super) fn scroll_down(&mut self, amount: usize) -> bool {
        self.scroll_by(-(amount as isize))
    }

    pub(super) fn scroll_top(&mut self) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        let screen = session.screen_mut();
        let top = screen.scrollback_len();
        if top == screen.scrollback() {
            return false;
        }
        screen.set_scrollback(top);
        if self.mode == TerminalMode::Normal {
            self.normal_cursor.0 = 0;
        }
        true
    }

    pub(super) fn scroll_to_bottom(&mut self) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        let screen = session.screen_mut();
        if screen.scrollback() == 0 {
            return false;
        }
        screen.set_scrollback(0);
        if self.mode == TerminalMode::Normal {
            self.normal_cursor.0 = screen.cursor().row;
        }
        true
    }

    pub(super) fn yank_line(&mut self) {
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let screen = session.screen();
        let row = self.normal_cursor.0.min(screen.rows().saturating_sub(1));
        let col = screen.cols().saturating_sub(1);
        let text = screen.contents_between(row, 0, row, col);
        SystemClipboard::new().write(ClipboardKind::Clipboard, text.trim_end());
        self.status = "yanked line".to_string();
    }
}
