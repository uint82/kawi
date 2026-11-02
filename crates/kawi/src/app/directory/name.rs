use std::ffi::{OsStr, OsString};

const HEX_DIGITS: [u8; 16] = *b"0123456789abcdef";

pub(super) fn escape_name(name: &OsStr) -> String {
    let mut text = String::with_capacity(name.len());
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        push_escaped_bytes(&mut text, name.as_bytes());
    }
    #[cfg(not(unix))]
    {
        push_escaped_str(&mut text, &name.to_string_lossy());
    }
    text
}

#[cfg(unix)]
fn push_escaped_bytes(text: &mut String, mut bytes: &[u8]) {
    while !bytes.is_empty() {
        match std::str::from_utf8(bytes) {
            Ok(valid) => {
                push_escaped_str(text, valid);
                return;
            }
            Err(error) => {
                let valid_up_to = error.valid_up_to();
                if let Ok(valid) = std::str::from_utf8(&bytes[..valid_up_to]) {
                    push_escaped_str(text, valid);
                }
                let invalid_length = error
                    .error_len()
                    .unwrap_or(bytes.len().saturating_sub(valid_up_to));
                for byte in &bytes[valid_up_to..valid_up_to + invalid_length] {
                    push_hex_escape(text, *byte);
                }
                bytes = &bytes[valid_up_to + invalid_length..];
            }
        }
    }
}

fn push_escaped_str(text: &mut String, value: &str) {
    for character in value.chars() {
        push_escaped_char(text, character);
    }
}

fn push_escaped_char(text: &mut String, character: char) {
    match character {
        '\\' => text.push_str("\\\\"),
        '\x07' => text.push_str("\\a"),
        '\x08' => text.push_str("\\b"),
        '\t' => text.push_str("\\t"),
        '\n' => text.push_str("\\n"),
        '\x0b' => text.push_str("\\v"),
        '\x0c' => text.push_str("\\f"),
        '\r' => text.push_str("\\r"),
        character if character.is_control() => push_escaped_control(text, character),
        character => text.push(character),
    }
}

fn push_escaped_control(text: &mut String, character: char) {
    let mut buffer = [0u8; 4];
    for byte in character.encode_utf8(&mut buffer).bytes() {
        push_hex_escape(text, byte);
    }
}

fn push_hex_escape(text: &mut String, byte: u8) {
    text.push('\\');
    text.push('x');
    text.push(HEX_DIGITS[usize::from(byte >> 4)] as char);
    text.push(HEX_DIGITS[usize::from(byte & 0x0f)] as char);
}

pub(super) fn unescape_name(text: &str) -> Result<OsString, String> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            push_plain_character(&mut bytes, character)?;
            continue;
        }
        let escaped = characters
            .next()
            .ok_or_else(|| "entry name ends with a backslash".to_string())?;
        match escaped {
            '\\' => push_escaped_byte(&mut bytes, b'\\')?,
            'a' => push_escaped_byte(&mut bytes, 0x07)?,
            'b' => push_escaped_byte(&mut bytes, 0x08)?,
            't' => push_escaped_byte(&mut bytes, b'\t')?,
            'n' => push_escaped_byte(&mut bytes, b'\n')?,
            'v' => push_escaped_byte(&mut bytes, 0x0b)?,
            'f' => push_escaped_byte(&mut bytes, 0x0c)?,
            'r' => push_escaped_byte(&mut bytes, b'\r')?,
            'x' => {
                let byte = parse_hex_byte(&mut characters)?;
                push_escaped_byte(&mut bytes, byte)?;
            }
            other => return Err(format!("invalid escape sequence \\{other} in entry name")),
        }
    }
    os_string_from_bytes(bytes)
}

fn push_plain_character(bytes: &mut Vec<u8>, character: char) -> Result<(), String> {
    if character == '\0' {
        return Err("entry name contains a NUL byte".to_string());
    }
    let mut buffer = [0u8; 4];
    bytes.extend_from_slice(character.encode_utf8(&mut buffer).as_bytes());
    Ok(())
}

fn push_escaped_byte(bytes: &mut Vec<u8>, byte: u8) -> Result<(), String> {
    match byte {
        0 => Err("entry name contains a NUL byte".to_string()),
        b'/' => Err("entry name cannot contain an escaped slash".to_string()),
        byte => {
            bytes.push(byte);
            Ok(())
        }
    }
}

fn parse_hex_byte(characters: &mut std::str::Chars<'_>) -> Result<u8, String> {
    let high = characters.next().and_then(hex_value);
    let low = characters.next().and_then(hex_value);
    let (Some(high), Some(low)) = (high, low) else {
        return Err("invalid hexadecimal escape in entry name".to_string());
    };
    Ok((high << 4) | low)
}

fn hex_value(character: char) -> Option<u8> {
    character
        .to_digit(16)
        .and_then(|value| u8::try_from(value).ok())
}

#[cfg(unix)]
fn os_string_from_bytes(bytes: Vec<u8>) -> Result<OsString, String> {
    use std::os::unix::ffi::OsStringExt;
    Ok(OsString::from_vec(bytes))
}

#[cfg(not(unix))]
fn os_string_from_bytes(bytes: Vec<u8>) -> Result<OsString, String> {
    String::from_utf8(bytes)
        .map(OsString::from)
        .map_err(|_| "entry name is not valid UTF-8".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    use std::os::unix::ffi::OsStrExt;

    #[test]
    fn plain_names_are_unchanged() {
        assert_eq!(escape_name(OsStr::new("a b.txt")), "a b.txt");
        assert_eq!(escape_name(OsStr::new("cafe\u{301}")), "cafe\u{301}");
    }

    #[test]
    fn special_characters_use_reversible_escapes() {
        assert_eq!(escape_name(OsStr::new("a\\b")), "a\\\\b");
        assert_eq!(escape_name(OsStr::new("a\nb\tc\rd")), "a\\nb\\tc\\rd");
        assert_eq!(escape_name(OsStr::new("a\x01b\x7f")), "a\\x01b\\x7f");
        assert_eq!(
            escape_name(OsStr::new("a\x07b\x08c\x0bd\x0ce")),
            "a\\ab\\bc\\vd\\fe"
        );
    }

    #[test]
    fn escaped_names_round_trip() {
        let names = [
            "plain", "a b", "a\\b", "a\nb", "a\tb", "a\rb", "a\x1bb", "a\u{85}b",
        ];
        for name in names {
            let escaped = escape_name(OsStr::new(name));
            let decoded = unescape_name(&escaped).unwrap();
            assert_eq!(decoded.as_os_str(), OsStr::new(name));
        }
    }

    #[test]
    fn invalid_escapes_are_rejected() {
        assert!(unescape_name("a\\q").is_err());
        assert!(unescape_name("a\\").is_err());
        assert!(unescape_name("a\\x1").is_err());
        assert!(unescape_name("a\\x2fb").is_err());
        assert!(unescape_name("a\\x00b").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn invalid_utf8_bytes_are_escaped() {
        let name = OsStr::from_bytes(b"a\xffb");
        assert_eq!(escape_name(name), "a\\xffb");
        let decoded = unescape_name("a\\xffb").unwrap();
        assert_eq!(decoded.as_os_str().as_bytes(), b"a\xffb");
    }
}
