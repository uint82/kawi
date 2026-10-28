mod config;
mod resolver;

use super::*;
use crate::command::CommandRegistry;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn plain(code: KeyCode) -> KeyChord {
    KeyChord::from_event(KeyEvent::new(code, KeyModifiers::NONE))
}

fn sequence(text: &str) -> KeySequence {
    KeySequence::parse(text, &[]).expect("sequence")
}

fn registry() -> CommandRegistry {
    CommandRegistry::new()
}

fn binding(
    registry: &CommandRegistry,
    name: &str,
    mode: KeyContext,
    keys: &str,
    source: BindingSource,
) -> Binding {
    Binding::new(
        registry.find(name).expect("command"),
        mode,
        sequence(keys),
        source,
    )
}

#[test]
fn parses_plain_and_named_keys() {
    let keys = KeySequence::parse("<C-w>gg<Space><Esc>", &[]).expect("sequence");
    assert_eq!(keys.len(), 5);
    assert!(keys[0].ctrl);
    assert_eq!(keys[0].code, KeyCodeSpec::Char('w'));
    assert_eq!(keys[1].code, KeyCodeSpec::Char('g'));
    assert_eq!(keys[3].code, KeyCodeSpec::Char(' '));
    assert_eq!(keys[4].code, KeyCodeSpec::Esc);
}

#[test]
fn parses_shift_and_function_keys() {
    let keys = KeySequence::parse("<S-Tab><F5>", &[]).expect("sequence");
    assert_eq!(keys[0].code, KeyCodeSpec::BackTab);
    assert_eq!(keys[1].code, KeyCodeSpec::F(5));
}

#[test]
fn parses_literal_angle_bracket() {
    let keys = KeySequence::parse("<lt>", &[]).expect("sequence");
    assert_eq!(keys[0].code, KeyCodeSpec::Char('<'));
}

#[test]
fn expands_leader() {
    let space = plain(KeyCode::Char(' '));
    let backslash = plain(KeyCode::Char('\\'));
    let keys = KeySequence::parse("<leader>ff", &[space, backslash]).expect("sequence");
    assert_eq!(keys.chords()[..2], [space, backslash]);
    assert_eq!(keys.len(), 4);
}

#[test]
fn rejects_malformed_sequences() {
    assert!(KeySequence::parse("", &[]).is_err());
    assert!(KeySequence::parse("<C-w", &[]).is_err());
    assert!(KeySequence::parse("<C->", &[]).is_err());
    assert!(KeySequence::parse("<leader>x", &[]).is_err());
}

#[test]
fn canonicalizes_shifted_characters() {
    let event = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT);
    let chord = KeyChord::from_event(event);
    assert_eq!(chord.code, KeyCodeSpec::Char('G'));
    assert!(!chord.shift);
}

#[test]
fn sequence_round_trips_through_display() {
    let keys = KeySequence::parse("<C-w><Space>A", &[]).expect("sequence");
    let text = keys.to_string();
    let parsed = KeySequence::parse(&text, &[]).expect("sequence");
    assert_eq!(keys, parsed);
}

#[test]
fn canonicalizes_shifted_lowercase_characters() {
    let event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::SHIFT);
    let chord = KeyChord::from_event(event);
    assert_eq!(chord.code, KeyCodeSpec::Char('A'));
    assert!(!chord.shift);
}
