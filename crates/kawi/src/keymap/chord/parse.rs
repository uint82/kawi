use super::{KeyChord, KeyCodeSpec, KeyParseError};

pub(super) fn parse_token(
    token: &str,
    leaders: &[KeyChord],
) -> Result<Vec<KeyChord>, KeyParseError> {
    if token.eq_ignore_ascii_case("leader") {
        let Some(leader) = leaders.first() else {
            return Err(KeyParseError::new("leader key is not configured"));
        };
        return Ok(vec![*leader]);
    }
    let mut parts: Vec<&str> = token.split('-').collect();
    let name = parts.pop().unwrap_or_default();
    if name.is_empty() {
        return Err(KeyParseError::new("missing key name"));
    }
    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    for part in parts {
        match part.to_ascii_lowercase().as_str() {
            "c" | "ctrl" | "control" => ctrl = true,
            "m" | "a" | "alt" | "meta" => alt = true,
            "s" | "shift" => shift = true,
            _ => return Err(KeyParseError::new(format!("unknown modifier {part}"))),
        }
    }
    let code = parse_key_name(name, shift)?;
    Ok(vec![KeyChord::new(code, ctrl, alt, shift)])
}

fn parse_key_name(name: &str, shift: bool) -> Result<KeyCodeSpec, KeyParseError> {
    let named = match name.to_ascii_lowercase().as_str() {
        "esc" | "escape" => Some(KeyCodeSpec::Esc),
        "cr" | "enter" | "return" => Some(KeyCodeSpec::Enter),
        "tab" => Some(KeyCodeSpec::Tab),
        "backtab" | "btab" => Some(KeyCodeSpec::BackTab),
        "bs" | "backspace" => Some(KeyCodeSpec::Backspace),
        "del" | "delete" => Some(KeyCodeSpec::Delete),
        "ins" | "insert" => Some(KeyCodeSpec::Insert),
        "home" => Some(KeyCodeSpec::Home),
        "end" => Some(KeyCodeSpec::End),
        "pageup" | "pgup" => Some(KeyCodeSpec::PageUp),
        "pagedown" | "pgdn" => Some(KeyCodeSpec::PageDown),
        "up" => Some(KeyCodeSpec::Up),
        "down" => Some(KeyCodeSpec::Down),
        "left" => Some(KeyCodeSpec::Left),
        "right" => Some(KeyCodeSpec::Right),
        "space" => Some(KeyCodeSpec::Char(' ')),
        "lt" => Some(KeyCodeSpec::Char('<')),
        "gt" => Some(KeyCodeSpec::Char('>')),
        "bar" => Some(KeyCodeSpec::Char('|')),
        "bslash" => Some(KeyCodeSpec::Char('\\')),
        other => function_key(other).or_else(|| single_character(other, shift)),
    };
    named.ok_or_else(|| KeyParseError::new(format!("unknown key {name}")))
}

fn function_key(name: &str) -> Option<KeyCodeSpec> {
    let number = name.strip_prefix('f')?.parse::<u8>().ok()?;
    (1..=24).contains(&number).then_some(KeyCodeSpec::F(number))
}

fn single_character(name: &str, shift: bool) -> Option<KeyCodeSpec> {
    let mut characters = name.chars();
    let character = characters.next()?;
    if characters.next().is_some() {
        return None;
    }
    let character = if shift && character.is_ascii_lowercase() {
        character.to_ascii_uppercase()
    } else {
        character
    };
    Some(KeyCodeSpec::Char(character))
}
