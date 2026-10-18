use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

const SPECIAL: char = '\u{80}';

pub(super) fn encode_key(key: KeyEvent) -> Option<String> {
    match key.code {
        KeyCode::Char(c) => Some(encode_char(c, key.modifiers)),
        KeyCode::Enter => Some("\r".to_string()),
        KeyCode::Tab => Some("\t".to_string()),
        KeyCode::BackTab => Some(special_key('T')),
        KeyCode::Backspace => Some("\u{7f}".to_string()),
        KeyCode::Esc => Some("\u{1b}".to_string()),
        KeyCode::Up => Some(special_key('U')),
        KeyCode::Down => Some(special_key('D')),
        KeyCode::Left => Some(special_key('L')),
        KeyCode::Right => Some(special_key('R')),
        KeyCode::Home => Some(special_key('H')),
        KeyCode::End => Some(special_key('E')),
        KeyCode::PageUp => Some(special_key('P')),
        KeyCode::PageDown => Some(special_key('N')),
        KeyCode::Insert => Some(special_key('I')),
        KeyCode::Delete => Some(special_key('X')),
        KeyCode::F(number) => Some(format!("{SPECIAL}F{number:02}")),
        _ => None,
    }
}

pub(super) fn decode_keys(text: &str) -> Vec<KeyEvent> {
    let mut keys = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == SPECIAL {
            if let Some(key) = decode_special(&mut chars) {
                keys.push(key);
            }
            continue;
        }
        keys.push(decode_char(c));
    }
    keys
}

fn special_key(code: char) -> String {
    format!("{SPECIAL}{code}")
}

fn encode_char(c: char, modifiers: KeyModifiers) -> String {
    if modifiers.contains(KeyModifiers::CONTROL) {
        if let Some(control) = control_char(c) {
            return control.to_string();
        }
    }
    if c == SPECIAL {
        return format!("{SPECIAL}{SPECIAL}");
    }
    c.to_string()
}

fn control_char(c: char) -> Option<char> {
    let byte = c as u32;
    if (0x20..=0x7e).contains(&byte) {
        char::from_u32(byte & 0x1f)
    } else {
        None
    }
}

fn decode_char(c: char) -> KeyEvent {
    let byte = c as u32;
    let (code, modifiers) = match byte {
        0x09 => (KeyCode::Tab, KeyModifiers::NONE),
        0x0a | 0x0d => (KeyCode::Enter, KeyModifiers::NONE),
        0x1b => (KeyCode::Esc, KeyModifiers::NONE),
        0x7f => (KeyCode::Backspace, KeyModifiers::NONE),
        0x01..=0x1a => (
            KeyCode::Char(letter_char(byte, 0x60)),
            KeyModifiers::CONTROL,
        ),
        0x1c..=0x1f => (
            KeyCode::Char(letter_char(byte, 0x40)),
            KeyModifiers::CONTROL,
        ),
        0x00 => (KeyCode::Char('@'), KeyModifiers::CONTROL),
        _ => (KeyCode::Char(c), KeyModifiers::NONE),
    };
    KeyEvent::new(code, modifiers)
}

fn letter_char(byte: u32, base: u32) -> char {
    char::from_u32(byte | base).unwrap_or(core::char::REPLACEMENT_CHARACTER)
}

fn decode_special(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<KeyEvent> {
    let code = chars.next()?;
    let key = match code {
        SPECIAL => KeyCode::Char(SPECIAL),
        'U' => KeyCode::Up,
        'D' => KeyCode::Down,
        'L' => KeyCode::Left,
        'R' => KeyCode::Right,
        'H' => KeyCode::Home,
        'E' => KeyCode::End,
        'P' => KeyCode::PageUp,
        'N' => KeyCode::PageDown,
        'I' => KeyCode::Insert,
        'X' => KeyCode::Delete,
        'T' => KeyCode::BackTab,
        'F' => {
            let tens = chars.next()?.to_digit(10)?;
            let ones = chars.next()?.to_digit(10)?;
            KeyCode::F((tens * 10 + ones) as u8)
        }
        _ => return None,
    };
    Some(KeyEvent::new(key, KeyModifiers::NONE))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    #[test]
    fn encodes_printable_characters_verbatim() {
        assert_eq!(encode_key(plain(KeyCode::Char('a'))).as_deref(), Some("a"));
        assert_eq!(encode_key(plain(KeyCode::Char(' '))).as_deref(), Some(" "));
        assert_eq!(encode_key(plain(KeyCode::Char('é'))).as_deref(), Some("é"));
    }

    #[test]
    fn encodes_control_and_named_keys() {
        assert_eq!(encode_key(plain(KeyCode::Esc)).as_deref(), Some("\u{1b}"));
        assert_eq!(encode_key(plain(KeyCode::Enter)).as_deref(), Some("\r"));
        assert_eq!(encode_key(plain(KeyCode::Tab)).as_deref(), Some("\t"));
        assert_eq!(
            encode_key(plain(KeyCode::Backspace)).as_deref(),
            Some("\u{7f}")
        );
        assert_eq!(encode_key(ctrl('c')).as_deref(), Some("\u{3}"));
        assert_eq!(encode_key(ctrl('[')).as_deref(), Some("\u{1b}"));
    }

    #[test]
    fn encodes_special_keys_behind_escape() {
        assert_eq!(encode_key(plain(KeyCode::Down)).as_deref(), Some("\u{80}D"));
        assert_eq!(
            encode_key(plain(KeyCode::PageUp)).as_deref(),
            Some("\u{80}P")
        );
        assert_eq!(
            encode_key(plain(KeyCode::F(5))).as_deref(),
            Some("\u{80}F05")
        );
    }

    #[test]
    fn round_trips_every_supported_key() {
        let keys = [
            plain(KeyCode::Char('x')),
            plain(KeyCode::Char('é')),
            plain(KeyCode::Esc),
            plain(KeyCode::Enter),
            plain(KeyCode::Tab),
            plain(KeyCode::BackTab),
            plain(KeyCode::Backspace),
            plain(KeyCode::Up),
            plain(KeyCode::Down),
            plain(KeyCode::Left),
            plain(KeyCode::Right),
            plain(KeyCode::Home),
            plain(KeyCode::End),
            plain(KeyCode::PageUp),
            plain(KeyCode::PageDown),
            plain(KeyCode::Insert),
            plain(KeyCode::Delete),
            plain(KeyCode::F(1)),
            plain(KeyCode::F(12)),
            ctrl('a'),
            ctrl('z'),
        ];
        let encoded: String = keys.iter().filter_map(|key| encode_key(*key)).collect();
        assert_eq!(decode_keys(&encoded), keys);
    }

    #[test]
    fn decodes_plain_text_as_characters() {
        let keys = decode_keys("2dw");
        assert_eq!(
            keys,
            vec![
                plain(KeyCode::Char('2')),
                plain(KeyCode::Char('d')),
                plain(KeyCode::Char('w'))
            ]
        );
    }

    #[test]
    fn decodes_literal_escape_character() {
        let keys = decode_keys(&format!("{SPECIAL}{SPECIAL}"));
        assert_eq!(keys, vec![plain(KeyCode::Char(SPECIAL))]);
    }

    #[test]
    fn ignores_unknown_special_codes() {
        let keys = decode_keys(&format!("a{SPECIAL}zb"));
        assert_eq!(
            keys,
            vec![plain(KeyCode::Char('a')), plain(KeyCode::Char('b'))]
        );
    }
}
