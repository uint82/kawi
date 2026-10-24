use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_completion::{Source, Trigger};

use super::super::lsp::LspUi;
use super::session;

pub(super) fn keeps_completion_open(lsp: &LspUi, key: KeyEvent) -> bool {
    session::is_continuation_key(key) || typed_trigger(lsp, key).is_some()
}

pub(super) fn trigger_for_key(lsp: &LspUi, key: KeyEvent, prefix: &str) -> Option<Trigger> {
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return None;
    }
    if key.code == KeyCode::Backspace {
        return (!prefix.is_empty()).then(Trigger::typing);
    }
    let KeyCode::Char(character) = key.code else {
        return None;
    };
    if let Some(trigger) = typed_trigger(lsp, key) {
        return Some(trigger);
    }
    prefix
        .chars()
        .last()
        .is_some_and(|last| last == character)
        .then(Trigger::typing)
}

fn typed_trigger(lsp: &LspUi, key: KeyEvent) -> Option<Trigger> {
    let KeyCode::Char(character) = key.code else {
        return None;
    };
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return None;
    }
    let capabilities = lsp.capabilities();
    capabilities
        .completion_triggers
        .iter()
        .any(|trigger| trigger == &character.to_string())
        .then(|| Trigger::character(character))
}

pub(crate) fn is_completion_key(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char(' ') | KeyCode::Char('@'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_completion_keys_require_control() {
        assert!(is_completion_key(KeyEvent::new(
            KeyCode::Char(' '),
            KeyModifiers::CONTROL
        )));
        assert!(!is_completion_key(KeyEvent::new(
            KeyCode::Char(' '),
            KeyModifiers::NONE
        )));
    }

    #[test]
    fn typing_continues_completion_after_identifier_characters() {
        assert!(session::is_continuation_key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::NONE
        )));
        assert!(!session::is_continuation_key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL
        )));
    }
}
