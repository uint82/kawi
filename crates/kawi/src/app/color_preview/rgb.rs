use super::color::{Rgba, Token};

const RGB: &[u8] = b"rgb";
const RGBA: &[u8] = b"rgba";
const OPEN: u8 = b'(';
const CLOSE: u8 = b')';
const SLASH: u8 = b'/';
const OPAQUE: u8 = 255;
const MAX_CHANNEL: f64 = 255.0;
const MAX_PERCENT: f64 = 100.0;
const PERCENT_SCALE: f64 = MAX_CHANNEL / MAX_PERCENT;

enum Component {
    Number(f64),
    Percent(f64),
}

struct Parts<'a> {
    red: &'a str,
    green: &'a str,
    blue: &'a str,
    alpha: Option<&'a str>,
}

pub(super) fn token_at(bytes: &[u8], index: usize) -> Option<Token> {
    let name_len = function_name_len(bytes, index)?;
    let open = index.saturating_add(name_len);
    if bytes.get(open) != Some(&OPEN) {
        return None;
    }
    let close = close_paren(bytes, open.saturating_add(1))?;
    let content = std::str::from_utf8(bytes.get(open.saturating_add(1)..close)?).ok()?;
    let parts = Parts::parse(content)?;
    let color = parts.to_color()?;
    Some(Token::new(
        color,
        close.saturating_sub(index).saturating_add(1),
    ))
}

fn function_name_len(bytes: &[u8], index: usize) -> Option<usize> {
    if bytes
        .get(index..index.saturating_add(RGBA.len()))?
        .eq_ignore_ascii_case(RGBA)
    {
        return Some(RGBA.len());
    }
    if bytes
        .get(index..index.saturating_add(RGB.len()))?
        .eq_ignore_ascii_case(RGB)
    {
        return Some(RGB.len());
    }
    None
}

fn close_paren(bytes: &[u8], start: usize) -> Option<usize> {
    bytes
        .get(start..)?
        .iter()
        .position(|byte| *byte == CLOSE)
        .map(|offset| start.saturating_add(offset))
}

impl<'a> Parts<'a> {
    fn parse(content: &'a str) -> Option<Self> {
        if content.contains(',') {
            Self::legacy(content)
        } else {
            Self::modern(content)
        }
    }

    fn legacy(content: &'a str) -> Option<Self> {
        let mut parts = content.split(',').map(str::trim);
        let parsed = Self {
            red: parts.next()?,
            green: parts.next()?,
            blue: parts.next()?,
            alpha: parts.next(),
        };
        parts.next().is_none().then_some(parsed)
    }

    fn modern(content: &'a str) -> Option<Self> {
        let slashes = content.bytes().filter(|byte| *byte == SLASH).count();
        if slashes > 1 {
            return None;
        }
        let mut parts = content
            .split(|character: char| character.is_ascii_whitespace() || character == '/')
            .filter(|part| !part.is_empty());
        let parsed = Self {
            red: parts.next()?,
            green: parts.next()?,
            blue: parts.next()?,
            alpha: parts.next(),
        };
        if parts.next().is_some() || parsed.alpha.is_some() != (slashes == 1) {
            return None;
        }
        Some(parsed)
    }

    fn to_color(&self) -> Option<Rgba> {
        let alpha = match self.alpha {
            Some(text) => alpha(text)?,
            None => OPAQUE,
        };
        Some(Rgba::new(
            channel(self.red)?,
            channel(self.green)?,
            channel(self.blue)?,
            alpha,
        ))
    }
}

fn channel(text: &str) -> Option<u8> {
    let value = match component(text)? {
        Component::Number(value) => value,
        Component::Percent(value) => value * PERCENT_SCALE,
    };
    Some(round_clamped(value, MAX_CHANNEL))
}

fn alpha(text: &str) -> Option<u8> {
    let value = match component(text)? {
        Component::Number(value) => value,
        Component::Percent(value) => value / MAX_PERCENT,
    };
    Some((value.clamp(0.0, 1.0) * MAX_CHANNEL).round() as u8)
}

fn component(text: &str) -> Option<Component> {
    let (number, percent) = match text.strip_suffix('%') {
        Some(number) => (number, true),
        None => (text, false),
    };
    if number.is_empty() {
        return None;
    }
    let value: f64 = number.parse().ok()?;
    if !value.is_finite() {
        return None;
    }
    Some(if percent {
        Component::Percent(value)
    } else {
        Component::Number(value)
    })
}

fn round_clamped(value: f64, maximum: f64) -> u8 {
    value.clamp(0.0, maximum).round() as u8
}

#[cfg(test)]
mod tests {
    use super::super::color::Rgba;
    use super::token_at;

    #[test]
    fn parses_modern_space_syntax() {
        assert_eq!(
            token_at(b"rgb(0 0 0 / 0.35)", 0).map(|token| token.color),
            Some(Rgba::new(0, 0, 0, 89))
        );
        assert_eq!(
            token_at(b"rgb(110 208 240 / 0.12)", 0).map(|token| token.color),
            Some(Rgba::new(110, 208, 240, 31))
        );
    }

    #[test]
    fn parses_legacy_comma_syntax() {
        assert!(token_at(b"rgb(255, 0, 0)", 0).is_some());
        assert!(token_at(b"rgba(255, 0, 0, 0.5)", 0).is_some());
    }

    #[test]
    fn parses_percent_channels_and_alpha() {
        assert_eq!(
            token_at(b"rgb(100% 0% 0% / 50%)", 0).map(|token| token.color),
            Some(Rgba::new(255, 0, 0, 128))
        );
    }

    #[test]
    fn clamps_out_of_range_components() {
        assert_eq!(
            token_at(b"rgb(-10 300 127.5 / 2)", 0).map(|token| token.color),
            Some(Rgba::new(0, 255, 128, 255))
        );
    }

    #[test]
    fn rejects_invalid_function_forms() {
        for value in [
            "rgb(0 0)",
            "rgb(0 0 0 0.5)",
            "rgb(0 0 0 /)",
            "rgb(0 0 0 / 0.5 / 0.5)",
            "rgb(0, 0, 0, 0.5, 1)",
            "rgba(0 0 0 0.5)",
        ] {
            assert!(token_at(value.as_bytes(), 0).is_none(), "{value}");
        }
    }

    #[test]
    fn accepts_uppercase_function_name() {
        assert!(token_at(b"RGB(0 0 0)", 0).is_some());
    }

    #[test]
    fn measures_the_full_token() {
        assert_eq!(
            token_at(b"rgb(0 0 0 / 0.35)", 0).map(|token| token.len),
            Some(17)
        );
    }
}
