use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use kawi_editor::Action;
use kawi_editor::Mode;

use crate::helix::HelixKeymap;
use crate::leader::LeaderPath;
use crate::vim::VimKeymap;

pub trait Keymap {
    fn map(&mut self, key: KeyEvent, mode: Mode) -> Action;

    fn mode_label(&self, mode: Mode) -> &'static str {
        mode.label()
    }

    fn operator_exits_visual(&self) -> bool;

    fn pending_text(&self) -> Option<&str> {
        None
    }

    fn pending_keys(&self) -> Option<&str> {
        None
    }

    fn leader_path(&self) -> Option<LeaderPath> {
        None
    }

    fn is_leader_prefix(&self, _key: KeyEvent, _mode: Mode) -> bool {
        false
    }

    fn reset_pending(&mut self) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavor {
    Vim,
    Helix,
}

impl Flavor {
    pub fn name(self) -> &'static str {
        match self {
            Self::Vim => "vim",
            Self::Helix => "helix",
        }
    }
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

    fn pending_text(&self) -> Option<&str> {
        match self {
            Self::Vim(inner) => inner.pending_text(),
            Self::Helix(inner) => inner.pending_text(),
        }
    }

    fn pending_keys(&self) -> Option<&str> {
        match self {
            Self::Vim(inner) => inner.pending_keys(),
            Self::Helix(inner) => inner.pending_keys(),
        }
    }

    fn leader_path(&self) -> Option<LeaderPath> {
        match self {
            Self::Vim(inner) => inner.leader_path(),
            Self::Helix(inner) => inner.leader_path(),
        }
    }

    fn is_leader_prefix(&self, key: KeyEvent, mode: Mode) -> bool {
        match self {
            Self::Vim(inner) => inner.is_leader_prefix(key, mode),
            Self::Helix(inner) => inner.is_leader_prefix(key, mode),
        }
    }

    fn reset_pending(&mut self) {
        match self {
            Self::Vim(inner) => inner.reset_pending(),
            Self::Helix(inner) => inner.reset_pending(),
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
    if let Some(action) = map_history_key(key) {
        return action;
    }
    match key.code {
        KeyCode::Esc => Action::CancelCommand,
        KeyCode::Enter => Action::ExecuteCommand,
        KeyCode::Backspace => Action::DeleteCommandChar,
        KeyCode::BackTab => Action::CompleteCommandBackward,
        KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
            Action::CompleteCommandBackward
        }
        KeyCode::Tab => Action::CompleteCommand,
        KeyCode::Char(c) => map_command_char(key, c),
        _ => Action::Noop,
    }
}

fn map_history_key(key: KeyEvent) -> Option<Action> {
    let plain = key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT;
    match key.code {
        KeyCode::Up | KeyCode::PageUp if plain => Some(Action::HistoryOlder),
        KeyCode::Down | KeyCode::PageDown if plain => Some(Action::HistoryNewer),
        KeyCode::Char('p') if key.modifiers == KeyModifiers::CONTROL => Some(Action::HistoryOlder),
        KeyCode::Char('n') if key.modifiers == KeyModifiers::CONTROL => Some(Action::HistoryNewer),
        _ => None,
    }
}

fn map_command_char(key: KeyEvent, c: char) -> Action {
    if has_ctrl_or_alt(key) {
        return Action::Noop;
    }
    Action::PushCommandChar(c)
}

pub fn map_search_mode(key: KeyEvent) -> Action {
    if let Some(action) = map_history_key(key) {
        return action;
    }
    match key.code {
        KeyCode::Esc => Action::CancelSearch,
        KeyCode::Enter => Action::ConfirmSearch,
        KeyCode::Backspace => Action::DeleteSearchChar,
        KeyCode::Char(c) => map_search_char(key, c),
        _ => Action::Noop,
    }
}

fn map_search_char(key: KeyEvent, c: char) -> Action {
    if has_ctrl_or_alt(key) {
        return Action::Noop;
    }
    Action::PushSearchChar(c)
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
