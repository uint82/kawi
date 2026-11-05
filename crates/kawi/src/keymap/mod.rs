mod binding;
mod chord;
pub(crate) mod config;
mod resolver;
#[cfg(test)]
mod tests;

pub(crate) use binding::{Binding, BindingSource, KeyContext};
pub(crate) use chord::{KeyChord, KeyCodeSpec, KeySequence};
pub(crate) use resolver::{KeymapResolver, ResolveOutcome};

pub(crate) fn has_reserved_prefix(keys: &KeySequence) -> bool {
    keys.first().is_some_and(|chord| {
        matches!(
            chord.code,
            KeyCodeSpec::Char(character) if character.is_ascii_digit() || character == '"'
        )
    })
}

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use kawi_editor::Action;
use kawi_editor::Mode;

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

    fn has_pending(&self) -> bool {
        false
    }

    fn set_leaders(&mut self, _leaders: Vec<KeyChord>) {}
}

pub(crate) fn default_leaders() -> Vec<KeyChord> {
    [
        KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Char('\\'), KeyModifiers::NONE),
    ]
    .into_iter()
    .map(KeyChord::from_event)
    .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavor {
    Vim,
}

impl Flavor {
    pub fn name(self) -> &'static str {
        match self {
            Self::Vim => "vim",
        }
    }
}

pub enum ActiveKeymap {
    Vim(VimKeymap),
}

impl ActiveKeymap {
    pub fn for_flavor(flavor: Flavor) -> Self {
        match flavor {
            Flavor::Vim => Self::Vim(VimKeymap::new()),
        }
    }
}

pub fn flavor_from_name(name: &str) -> Option<Flavor> {
    match name.trim() {
        "vim" => Some(Flavor::Vim),
        _ => None,
    }
}

impl Keymap for ActiveKeymap {
    fn map(&mut self, key: KeyEvent, mode: Mode) -> Action {
        match self {
            Self::Vim(inner) => inner.map(key, mode),
        }
    }

    fn mode_label(&self, mode: Mode) -> &'static str {
        match self {
            Self::Vim(inner) => inner.mode_label(mode),
        }
    }

    fn operator_exits_visual(&self) -> bool {
        match self {
            Self::Vim(inner) => inner.operator_exits_visual(),
        }
    }

    fn pending_text(&self) -> Option<&str> {
        match self {
            Self::Vim(inner) => inner.pending_text(),
        }
    }

    fn pending_keys(&self) -> Option<&str> {
        match self {
            Self::Vim(inner) => inner.pending_keys(),
        }
    }

    fn leader_path(&self) -> Option<LeaderPath> {
        match self {
            Self::Vim(inner) => inner.leader_path(),
        }
    }

    fn is_leader_prefix(&self, key: KeyEvent, mode: Mode) -> bool {
        match self {
            Self::Vim(inner) => inner.is_leader_prefix(key, mode),
        }
    }

    fn reset_pending(&mut self) {
        match self {
            Self::Vim(inner) => inner.reset_pending(),
        }
    }

    fn has_pending(&self) -> bool {
        match self {
            Self::Vim(inner) => inner.has_pending(),
        }
    }

    fn set_leaders(&mut self, leaders: Vec<KeyChord>) {
        match self {
            Self::Vim(inner) => inner.set_leaders(leaders),
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
