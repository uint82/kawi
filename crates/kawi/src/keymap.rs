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

pub fn flavor_from_name(name: &str) -> Option<Flavor> {
    match name.trim() {
        "vim" => Some(Flavor::Vim),
        "helix" => Some(Flavor::Helix),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_resolve_flavors() {
        assert_eq!(flavor_from_name("vim"), Some(Flavor::Vim));
        assert_eq!(flavor_from_name("helix"), Some(Flavor::Helix));
    }

    #[test]
    fn unknown_name_resolves_none() {
        assert_eq!(flavor_from_name("personal"), None);
        assert_eq!(flavor_from_name(""), None);
    }

    #[test]
    fn interrupt_becomes_plain_esc() {
        let interrupt = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(
            normalize_interrupt(interrupt),
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)
        );
    }

    #[test]
    fn other_keys_pass_through() {
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
        assert_eq!(normalize_interrupt(key), key);
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

pub fn normalize_interrupt(key: KeyEvent) -> KeyEvent {
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
        return KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    }
    key
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
