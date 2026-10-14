use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use kawi_editor::Action;
use kawi_editor::Mode;

use crate::helix::HelixKeymap;
use crate::vim::VimKeymap;

pub trait Keymap {
    fn map(&mut self, key: KeyEvent, mode: Mode) -> Action;

    fn mode_label(&self, mode: Mode) -> &'static str {
        mode.label()
    }

    fn operator_exits_visual(&self) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavor {
    Vim,
    Helix,
}

pub enum ActiveKeymap {
    Vim(VimKeymap),
    Helix(HelixKeymap),
}

impl ActiveKeymap {
    pub fn for_flavor(flavor: Flavor) -> Self {
        match flavor {
            Flavor::Vim => Self::Vim(VimKeymap::new()),
            Flavor::Helix => Self::Helix(HelixKeymap::new()),
        }
    }
}

impl Keymap for ActiveKeymap {
    fn map(&mut self, key: KeyEvent, mode: Mode) -> Action {
        match self {
            Self::Vim(inner) => inner.map(key, mode),
            Self::Helix(inner) => inner.map(key, mode),
        }
    }

    fn mode_label(&self, mode: Mode) -> &'static str {
        match self {
            Self::Vim(inner) => inner.mode_label(mode),
            Self::Helix(inner) => inner.mode_label(mode),
        }
    }

    fn operator_exits_visual(&self) -> bool {
        match self {
            Self::Vim(inner) => inner.operator_exits_visual(),
            Self::Helix(inner) => inner.operator_exits_visual(),
        }
    }
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

pub fn map_command_mode(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => Action::CancelCommand,
        KeyCode::Enter => Action::ExecuteCommand,
        KeyCode::Backspace => Action::DeleteCommandChar,
        KeyCode::Char(c) => map_command_char(key, c),
        _ => Action::Noop,
    }
}

fn map_command_char(key: KeyEvent, c: char) -> Action {
    if has_ctrl_or_alt(key) {
        return Action::Noop;
    }
    Action::PushCommandChar(c)
}
