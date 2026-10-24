use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_terminal::{encode_key, encode_paste, KeyModes};

use super::{TerminalKey, TerminalMode, TerminalUi};

impl TerminalUi {
    pub fn handle_key(&mut self, key: KeyEvent, rows: usize) -> TerminalKey {
        match self.mode {
            TerminalMode::Input => self.handle_input_key(key),
            TerminalMode::Normal => self.handle_normal_key(key, rows),
        }
    }

    pub fn handle_paste(&mut self, text: &str) -> TerminalKey {
        if self.mode != TerminalMode::Input {
            return TerminalKey::Consumed;
        }
        self.escape_pending = false;
        let modes = self.key_modes();
        self.write(&encode_paste(text, modes));
        TerminalKey::Consumed
    }

    fn handle_input_key(&mut self, key: KeyEvent) -> TerminalKey {
        if self.escape_pending {
            self.escape_pending = false;
            if is_control(key, 'n') {
                self.enter_normal();
                return TerminalKey::Consumed;
            }
            if is_control(key, 'o') {
                self.enter_normal();
                self.one_shot = true;
                return TerminalKey::Consumed;
            }
            self.write_key(key);
            return TerminalKey::Consumed;
        }
        if is_escape_chord(key) {
            self.escape_pending = true;
            return TerminalKey::Consumed;
        }
        self.write_key(key);
        TerminalKey::Consumed
    }

    fn handle_normal_key(&mut self, key: KeyEvent, rows: usize) -> TerminalKey {
        let result = match key.code {
            KeyCode::Char('i' | 'a' | 'I' | 'A') if is_plain(key) && self.is_running() => {
                self.mode = TerminalMode::Input;
                self.one_shot = false;
                self.scroll_to_bottom();
                TerminalKey::Consumed
            }
            KeyCode::Char('q') if is_plain(key) => TerminalKey::ClosePanel,
            KeyCode::Esc => TerminalKey::FocusEditor,
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll_down(1);
                TerminalKey::Consumed
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll_up(1);
                TerminalKey::Consumed
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_down(half_page(rows));
                TerminalKey::Consumed
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_up(half_page(rows));
                TerminalKey::Consumed
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_down(rows);
                TerminalKey::Consumed
            }
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_up(rows);
                TerminalKey::Consumed
            }
            KeyCode::PageDown => {
                self.scroll_down(rows);
                TerminalKey::Consumed
            }
            KeyCode::PageUp => {
                self.scroll_up(rows);
                TerminalKey::Consumed
            }
            KeyCode::Char('g') => {
                self.scroll_top();
                TerminalKey::Consumed
            }
            KeyCode::Char('G') => {
                self.scroll_to_bottom();
                TerminalKey::Consumed
            }
            KeyCode::Char('y') => {
                self.yank_line();
                TerminalKey::Consumed
            }
            _ => TerminalKey::Consumed,
        };
        if self.one_shot && result == TerminalKey::Consumed && self.mode == TerminalMode::Normal {
            self.mode = TerminalMode::Input;
            self.one_shot = false;
        }
        result
    }

    fn enter_normal(&mut self) {
        self.mode = TerminalMode::Normal;
        self.escape_pending = false;
        self.one_shot = false;
        if let Some(session) = self.session.as_ref() {
            let cursor = session.screen().cursor();
            self.normal_cursor = (cursor.row, cursor.col);
        }
    }

    fn write_key(&mut self, key: KeyEvent) {
        let modes = self.key_modes();
        self.write(&encode_key(key, modes));
    }

    fn write(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        let Some(session) = self.session.as_mut() else {
            return;
        };
        if let Err(error) = session.write(bytes) {
            self.status = format!("write failed: {error}");
        }
    }

    fn key_modes(&self) -> KeyModes {
        let Some(session) = self.session.as_ref() else {
            return KeyModes::default();
        };
        KeyModes {
            application_cursor: session.screen().application_cursor(),
            bracketed_paste: session.screen().bracketed_paste(),
        }
    }
}

fn is_control(key: KeyEvent, character: char) -> bool {
    key.code == KeyCode::Char(character) && key.modifiers.contains(KeyModifiers::CONTROL)
}

fn is_escape_chord(key: KeyEvent) -> bool {
    is_control(key, '\\') || is_control(key, '4')
}

fn is_plain(key: KeyEvent) -> bool {
    !key.modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
}

fn half_page(rows: usize) -> usize {
    rows.max(1) / 2
}
