use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyModes {
    pub application_cursor: bool,
    pub bracketed_paste: bool,
}

pub fn encode_key(key: KeyEvent, modes: KeyModes) -> Vec<u8> {
    if key.kind == KeyEventKind::Release {
        return Vec::new();
    }
    match key.code {
        KeyCode::Char(character) => encode_character(character, key.modifiers),
        KeyCode::Null => vec![0],
        KeyCode::Enter => with_alt(key.modifiers, b"\r".to_vec()),
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => b"\x1b[Z".to_vec(),
        KeyCode::Tab => with_alt(key.modifiers, b"\t".to_vec()),
        KeyCode::Backspace => {
            let byte = if key.modifiers.contains(KeyModifiers::CONTROL) {
                b"\x08".to_vec()
            } else {
                b"\x7f".to_vec()
            };
            with_alt(key.modifiers, byte)
        }
        KeyCode::Esc => with_alt(key.modifiers, b"\x1b".to_vec()),
        KeyCode::Up => cursor_key(b'A', key.modifiers, modes.application_cursor),
        KeyCode::Down => cursor_key(b'B', key.modifiers, modes.application_cursor),
        KeyCode::Right => cursor_key(b'C', key.modifiers, modes.application_cursor),
        KeyCode::Left => cursor_key(b'D', key.modifiers, modes.application_cursor),
        KeyCode::Home => cursor_key(b'H', key.modifiers, modes.application_cursor),
        KeyCode::End => cursor_key(b'F', key.modifiers, modes.application_cursor),
        KeyCode::Insert => tilde_key(2, key.modifiers),
        KeyCode::Delete => tilde_key(3, key.modifiers),
        KeyCode::PageUp => tilde_key(5, key.modifiers),
        KeyCode::PageDown => tilde_key(6, key.modifiers),
        KeyCode::F(number) => function_key(number, key.modifiers),
        _ => Vec::new(),
    }
}

pub fn encode_paste(text: &str, modes: KeyModes) -> Vec<u8> {
    if !modes.bracketed_paste {
        return text.as_bytes().to_vec();
    }
    let mut bytes = Vec::with_capacity(text.len().saturating_add(12));
    bytes.extend_from_slice(b"\x1b[200~");
    bytes.extend_from_slice(text.as_bytes());
    bytes.extend_from_slice(b"\x1b[201~");
    bytes
}

fn encode_character(character: char, modifiers: KeyModifiers) -> Vec<u8> {
    if modifiers.contains(KeyModifiers::CONTROL) {
        if let Some(byte) = control_byte(character) {
            return with_alt(modifiers, vec![byte]);
        }
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(character.encode_utf8(&mut [0; 4]).as_bytes());
    with_alt(modifiers, bytes)
}

fn control_byte(character: char) -> Option<u8> {
    let lower = character.to_ascii_lowercase();
    match lower {
        '@' | ' ' | '2' => Some(0x00),
        'a'..='z' => Some(lower as u8 - b'a' + 1),
        '[' | '3' => Some(0x1b),
        '\\' | '4' => Some(0x1c),
        ']' | '5' => Some(0x1d),
        '^' | '6' => Some(0x1e),
        '_' | '7' => Some(0x1f),
        '?' | '8' => Some(0x7f),
        _ => None,
    }
}

fn with_alt(modifiers: KeyModifiers, bytes: Vec<u8>) -> Vec<u8> {
    if !modifiers.contains(KeyModifiers::ALT) {
        return bytes;
    }
    let mut prefixed = Vec::with_capacity(bytes.len().saturating_add(1));
    prefixed.push(0x1b);
    prefixed.extend_from_slice(&bytes);
    prefixed
}

fn cursor_key(letter: u8, modifiers: KeyModifiers, application: bool) -> Vec<u8> {
    match modifier_parameter(modifiers) {
        Some(parameter) => format!("\x1b[1;{parameter}{}", letter as char).into_bytes(),
        None if application => vec![0x1b, b'O', letter],
        None => vec![0x1b, b'[', letter],
    }
}

fn tilde_key(code: u8, modifiers: KeyModifiers) -> Vec<u8> {
    match modifier_parameter(modifiers) {
        Some(parameter) => format!("\x1b[{code};{parameter}~").into_bytes(),
        None => format!("\x1b[{code}~").into_bytes(),
    }
}

fn function_key(number: u8, modifiers: KeyModifiers) -> Vec<u8> {
    match number {
        1..=4 => {
            let letter = b'P' + (number - 1);
            match modifier_parameter(modifiers) {
                Some(parameter) => format!("\x1b[1;{parameter}{}", letter as char).into_bytes(),
                None => vec![0x1b, b'O', letter],
            }
        }
        5..=12 => {
            let code = match number {
                5 => 15,
                6 => 17,
                7 => 18,
                8 => 19,
                9 => 20,
                10 => 21,
                11 => 23,
                _ => 24,
            };
            tilde_key(code, modifiers)
        }
        _ => Vec::new(),
    }
}

fn modifier_parameter(modifiers: KeyModifiers) -> Option<u8> {
    let parameter = 1
        + u8::from(modifiers.contains(KeyModifiers::SHIFT))
        + u8::from(modifiers.contains(KeyModifiers::ALT)) * 2
        + u8::from(modifiers.contains(KeyModifiers::CONTROL)) * 4;
    (parameter > 1).then_some(parameter)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    fn plain(code: KeyCode) -> KeyEvent {
        key(code, KeyModifiers::NONE)
    }

    #[test]
    fn encodes_plain_characters() {
        assert_eq!(
            encode_key(plain(KeyCode::Char('a')), KeyModes::default()),
            b"a"
        );
        assert_eq!(
            encode_key(plain(KeyCode::Char('é')), KeyModes::default()),
            "é".as_bytes()
        );
    }

    #[test]
    fn encodes_control_characters() {
        assert_eq!(
            encode_key(
                key(KeyCode::Char('c'), KeyModifiers::CONTROL),
                KeyModes::default()
            ),
            vec![0x03]
        );
        assert_eq!(
            encode_key(
                key(KeyCode::Char('\\'), KeyModifiers::CONTROL),
                KeyModes::default()
            ),
            vec![0x1c]
        );
        assert_eq!(
            encode_key(
                key(KeyCode::Char('4'), KeyModifiers::CONTROL),
                KeyModes::default()
            ),
            vec![0x1c]
        );
        assert_eq!(
            encode_key(
                key(KeyCode::Char(' '), KeyModifiers::CONTROL),
                KeyModes::default()
            ),
            vec![0x00]
        );
    }

    #[test]
    fn encodes_alt_characters_with_escape_prefix() {
        assert_eq!(
            encode_key(
                key(KeyCode::Char('x'), KeyModifiers::ALT),
                KeyModes::default()
            ),
            b"\x1bx"
        );
    }

    #[test]
    fn encodes_enter_tab_and_backspace() {
        assert_eq!(
            encode_key(plain(KeyCode::Enter), KeyModes::default()),
            b"\r"
        );
        assert_eq!(encode_key(plain(KeyCode::Tab), KeyModes::default()), b"\t");
        assert_eq!(
            encode_key(plain(KeyCode::Backspace), KeyModes::default()),
            vec![0x7f]
        );
        assert_eq!(
            encode_key(key(KeyCode::Tab, KeyModifiers::SHIFT), KeyModes::default()),
            b"\x1b[Z"
        );
    }

    #[test]
    fn encodes_cursor_keys_in_both_modes() {
        assert_eq!(
            encode_key(plain(KeyCode::Up), KeyModes::default()),
            b"\x1b[A"
        );
        assert_eq!(
            encode_key(
                plain(KeyCode::Up),
                KeyModes {
                    application_cursor: true,
                    bracketed_paste: false,
                }
            ),
            b"\x1bOA"
        );
        assert_eq!(
            encode_key(
                key(KeyCode::Left, KeyModifiers::CONTROL),
                KeyModes::default()
            ),
            b"\x1b[1;5D"
        );
    }

    #[test]
    fn encodes_home_and_end_in_both_modes() {
        assert_eq!(
            encode_key(plain(KeyCode::Home), KeyModes::default()),
            b"\x1b[H"
        );
        assert_eq!(
            encode_key(
                plain(KeyCode::End),
                KeyModes {
                    application_cursor: true,
                    bracketed_paste: false,
                }
            ),
            b"\x1bOF"
        );
    }

    #[test]
    fn encodes_editing_and_paging_keys() {
        assert_eq!(
            encode_key(plain(KeyCode::Insert), KeyModes::default()),
            b"\x1b[2~"
        );
        assert_eq!(
            encode_key(plain(KeyCode::Delete), KeyModes::default()),
            b"\x1b[3~"
        );
        assert_eq!(
            encode_key(plain(KeyCode::PageUp), KeyModes::default()),
            b"\x1b[5~"
        );
        assert_eq!(
            encode_key(plain(KeyCode::PageDown), KeyModes::default()),
            b"\x1b[6~"
        );
    }

    #[test]
    fn encodes_function_keys() {
        assert_eq!(
            encode_key(plain(KeyCode::F(1)), KeyModes::default()),
            b"\x1bOP"
        );
        assert_eq!(
            encode_key(plain(KeyCode::F(5)), KeyModes::default()),
            b"\x1b[15~"
        );
        assert_eq!(
            encode_key(plain(KeyCode::F(12)), KeyModes::default()),
            b"\x1b[24~"
        );
    }

    #[test]
    fn encodes_bracketed_paste() {
        assert_eq!(encode_paste("abc", KeyModes::default()), b"abc".to_vec());
        assert_eq!(
            encode_paste(
                "abc",
                KeyModes {
                    application_cursor: false,
                    bracketed_paste: true,
                }
            ),
            b"\x1b[200~abc\x1b[201~".to_vec()
        );
    }
}
