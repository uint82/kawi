use super::color::{Rgba, Token};

const HASH: u8 = b'#';
const MAX_DIGITS: usize = 8;
const OPAQUE: u8 = 255;
const HEX_RADIX: u8 = 16;
const NIBBLE_SCALE: u8 = 17;

pub(super) fn token_at(bytes: &[u8], index: usize) -> Option<Token> {
    if bytes.get(index) != Some(&HASH) {
        return None;
    }
    let digits_start = index.saturating_add(1);
    let digits_len = hex_digits(bytes, digits_start);
    if digits_len > MAX_DIGITS {
        return None;
    }
    let digits = bytes.get(digits_start..digits_start.saturating_add(digits_len))?;
    let color = parse_digits(digits)?;
    Some(Token::new(color, digits_len.saturating_add(1)))
}

fn hex_digits(bytes: &[u8], start: usize) -> usize {
    bytes
        .get(start..)
        .unwrap_or_default()
        .iter()
        .take(MAX_DIGITS.saturating_add(1))
        .take_while(|byte| byte.is_ascii_hexdigit())
        .count()
}

fn parse_digits(digits: &[u8]) -> Option<Rgba> {
    match digits.len() {
        3 => from_nibbles(digits, false),
        4 => from_nibbles(digits, true),
        6 => from_pairs(digits, false),
        8 => from_pairs(digits, true),
        _ => None,
    }
}

fn from_nibbles(digits: &[u8], has_alpha: bool) -> Option<Rgba> {
    Some(Rgba::new(
        expand_nibble(*digits.first()?)?,
        expand_nibble(*digits.get(1)?)?,
        expand_nibble(*digits.get(2)?)?,
        if has_alpha {
            expand_nibble(*digits.get(3)?)?
        } else {
            OPAQUE
        },
    ))
}

fn from_pairs(digits: &[u8], has_alpha: bool) -> Option<Rgba> {
    Some(Rgba::new(
        byte(digits, 0)?,
        byte(digits, 2)?,
        byte(digits, 4)?,
        if has_alpha { byte(digits, 6)? } else { OPAQUE },
    ))
}

fn expand_nibble(digit: u8) -> Option<u8> {
    hex_value(digit).map(|value| value * NIBBLE_SCALE)
}

fn byte(digits: &[u8], index: usize) -> Option<u8> {
    let high = hex_value(*digits.get(index)?)?;
    let low = hex_value(*digits.get(index.saturating_add(1))?)?;
    Some(high * HEX_RADIX + low)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::token_at;

    #[test]
    fn parses_short_and_long_hex_colors() {
        assert_eq!(token_at(b"#abc", 0).map(|token| token.len), Some(4));
        assert_eq!(token_at(b"#abcd", 0).map(|token| token.len), Some(5));
        assert_eq!(token_at(b"#aabbcc", 0).map(|token| token.len), Some(7));
        assert_eq!(token_at(b"#aabbccdd", 0).map(|token| token.len), Some(9));
    }

    #[test]
    fn rejects_invalid_lengths() {
        for value in ["#ab", "#abcde", "#abcdef0"] {
            assert!(token_at(value.as_bytes(), 0).is_none(), "{value}");
        }
    }

    #[test]
    fn parses_uppercase_digits() {
        assert!(token_at(b"#AaBbCc", 0).is_some());
    }
}
