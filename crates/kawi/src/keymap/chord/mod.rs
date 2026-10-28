use std::fmt;
use std::ops::Deref;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

mod parse;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum KeyCodeSpec {
    Char(char),
    Esc,
    Enter,
    Tab,
    BackTab,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
    F(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct KeyChord {
    pub(crate) code: KeyCodeSpec,
    pub(crate) ctrl: bool,
    pub(crate) alt: bool,
    pub(crate) shift: bool,
}

impl KeyChord {
    pub(crate) fn from_event(event: KeyEvent) -> Self {
        let code = match event.code {
            KeyCode::Char(character) => KeyCodeSpec::Char(character),
            KeyCode::Esc => KeyCodeSpec::Esc,
            KeyCode::Enter => KeyCodeSpec::Enter,
            KeyCode::Tab => KeyCodeSpec::Tab,
            KeyCode::BackTab => KeyCodeSpec::BackTab,
            KeyCode::Backspace => KeyCodeSpec::Backspace,
            KeyCode::Delete => KeyCodeSpec::Delete,
            KeyCode::Insert => KeyCodeSpec::Insert,
            KeyCode::Home => KeyCodeSpec::Home,
            KeyCode::End => KeyCodeSpec::End,
            KeyCode::PageUp => KeyCodeSpec::PageUp,
            KeyCode::PageDown => KeyCodeSpec::PageDown,
            KeyCode::Up => KeyCodeSpec::Up,
            KeyCode::Down => KeyCodeSpec::Down,
            KeyCode::Left => KeyCodeSpec::Left,
            KeyCode::Right => KeyCodeSpec::Right,
            KeyCode::F(number) => KeyCodeSpec::F(number),
            _ => KeyCodeSpec::Char('\0'),
        };
        Self::new(
            code,
            event.modifiers.contains(KeyModifiers::CONTROL),
            event.modifiers.contains(KeyModifiers::ALT),
            event.modifiers.contains(KeyModifiers::SHIFT),
        )
    }

    pub(crate) fn to_event(self) -> KeyEvent {
        let mut modifiers = KeyModifiers::NONE;
        if self.ctrl {
            modifiers.insert(KeyModifiers::CONTROL);
        }
        if self.alt {
            modifiers.insert(KeyModifiers::ALT);
        }
        if self.shift {
            modifiers.insert(KeyModifiers::SHIFT);
        }
        KeyEvent::new(self.code.to_key_code(), modifiers)
    }

    pub(super) fn new(code: KeyCodeSpec, ctrl: bool, alt: bool, shift: bool) -> Self {
        let (code, shift) = if shift {
            match code {
                KeyCodeSpec::Char(character) if character.is_ascii_lowercase() => {
                    (KeyCodeSpec::Char(character.to_ascii_uppercase()), false)
                }
                KeyCodeSpec::Char(_) | KeyCodeSpec::BackTab => (code, false),
                KeyCodeSpec::Tab => (KeyCodeSpec::BackTab, false),
                _ => (code, true),
            }
        } else {
            (code, false)
        };
        Self {
            code,
            ctrl,
            alt,
            shift,
        }
    }

    fn display(self) -> String {
        let mut text = String::new();
        if self.ctrl {
            text.push_str("C-");
        }
        if self.alt {
            text.push_str("M-");
        }
        if self.shift {
            text.push_str("S-");
        }
        if text.is_empty() {
            return plain_label(self.code);
        }
        format!("<{text}{}>", key_name(self.code))
    }
}

impl KeyCodeSpec {
    fn to_key_code(self) -> KeyCode {
        match self {
            Self::Char(character) => KeyCode::Char(character),
            Self::Esc => KeyCode::Esc,
            Self::Enter => KeyCode::Enter,
            Self::Tab => KeyCode::Tab,
            Self::BackTab => KeyCode::BackTab,
            Self::Backspace => KeyCode::Backspace,
            Self::Delete => KeyCode::Delete,
            Self::Insert => KeyCode::Insert,
            Self::Home => KeyCode::Home,
            Self::End => KeyCode::End,
            Self::PageUp => KeyCode::PageUp,
            Self::PageDown => KeyCode::PageDown,
            Self::Up => KeyCode::Up,
            Self::Down => KeyCode::Down,
            Self::Left => KeyCode::Left,
            Self::Right => KeyCode::Right,
            Self::F(number) => KeyCode::F(number),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub(crate) struct KeySequence(Vec<KeyChord>);

impl KeySequence {
    pub(crate) fn parse(text: &str, leaders: &[KeyChord]) -> Result<Self, KeyParseError> {
        if text.is_empty() {
            return Err(KeyParseError::new("empty key sequence"));
        }
        let characters: Vec<char> = text.chars().collect();
        let mut chords = Vec::new();
        let mut index = 0;
        while index < characters.len() {
            let character = characters[index];
            if character != '<' {
                chords.push(KeyChord::new(
                    KeyCodeSpec::Char(character),
                    false,
                    false,
                    false,
                ));
                index += 1;
                continue;
            }
            let Some(end) = characters[index + 1..]
                .iter()
                .position(|character| *character == '>')
                .map(|offset| index + 1 + offset)
            else {
                return Err(KeyParseError::new("unterminated key notation"));
            };
            let token: String = characters[index + 1..end].iter().collect();
            chords.extend(parse::parse_token(&token, leaders)?);
            index = end + 1;
        }
        if chords.is_empty() {
            return Err(KeyParseError::new("empty key sequence"));
        }
        Ok(Self(chords))
    }

    pub(crate) fn from_chords(chords: Vec<KeyChord>) -> Self {
        Self(chords)
    }

    pub(crate) fn chords(&self) -> &[KeyChord] {
        &self.0
    }

    pub(crate) fn starts_with(&self, prefix: &[KeyChord]) -> bool {
        self.0.starts_with(prefix)
    }
}

impl Deref for KeySequence {
    type Target = [KeyChord];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for KeySequence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for chord in &self.0 {
            formatter.write_str(&chord.display())?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeyParseError {
    message: String,
}

impl KeyParseError {
    pub(super) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for KeyParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for KeyParseError {}

fn plain_label(code: KeyCodeSpec) -> String {
    match code {
        KeyCodeSpec::Char(' ') => "<Space>".to_string(),
        KeyCodeSpec::Char('<') => "<lt>".to_string(),
        KeyCodeSpec::Char(character) => character.to_string(),
        code => format!("<{}>", key_name(code)),
    }
}

fn key_name(code: KeyCodeSpec) -> String {
    match code {
        KeyCodeSpec::Char(' ') => "Space".to_string(),
        KeyCodeSpec::Char('<') => "lt".to_string(),
        KeyCodeSpec::Char(character) => character.to_string(),
        KeyCodeSpec::Esc => "Esc".to_string(),
        KeyCodeSpec::Enter => "CR".to_string(),
        KeyCodeSpec::Tab => "Tab".to_string(),
        KeyCodeSpec::BackTab => "S-Tab".to_string(),
        KeyCodeSpec::Backspace => "BS".to_string(),
        KeyCodeSpec::Delete => "Del".to_string(),
        KeyCodeSpec::Insert => "Ins".to_string(),
        KeyCodeSpec::Home => "Home".to_string(),
        KeyCodeSpec::End => "End".to_string(),
        KeyCodeSpec::PageUp => "PageUp".to_string(),
        KeyCodeSpec::PageDown => "PageDown".to_string(),
        KeyCodeSpec::Up => "Up".to_string(),
        KeyCodeSpec::Down => "Down".to_string(),
        KeyCodeSpec::Left => "Left".to_string(),
        KeyCodeSpec::Right => "Right".to_string(),
        KeyCodeSpec::F(number) => format!("F{number}"),
    }
}
