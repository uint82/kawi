use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use kawi_editor::Action;
use kawi_editor::Mode;

pub trait Keymap {
    fn map(&mut self, key: KeyEvent, mode: Mode) -> Action;
}

pub fn is_ctrl_quit(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char('q') | KeyCode::Char('c'))
}

pub fn is_save_key(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('s'))
}

pub fn has_ctrl_or_alt(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::ALT)
}
