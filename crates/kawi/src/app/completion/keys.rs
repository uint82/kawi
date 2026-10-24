use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeyBinding {
    code: KeyCode,
    modifiers: KeyModifiers,
}

impl KeyBinding {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        let mut modifiers = KeyModifiers::NONE;
        let mut name = value.trim().to_lowercase();
        while let Some((prefix, rest)) = name.split_once('-') {
            let modifier = match prefix {
                "c" | "ctrl" | "control" => KeyModifiers::CONTROL,
                "a" | "alt" | "meta" => KeyModifiers::ALT,
                "s" | "shift" => KeyModifiers::SHIFT,
                _ => break,
            };
            modifiers.insert(modifier);
            name = rest.to_string();
        }
        let code = key_code(&name)?;
        Some(Self { code, modifiers })
    }

    pub(crate) fn control(character: char) -> Self {
        Self {
            code: KeyCode::Char(character),
            modifiers: KeyModifiers::CONTROL,
        }
    }

    pub(crate) fn event(self) -> KeyEvent {
        KeyEvent::new(self.code, self.modifiers)
    }

    pub(crate) fn name(self) -> String {
        let mut name = String::new();
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            name.push_str("ctrl-");
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            name.push_str("alt-");
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            name.push_str("shift-");
        }
        name.push_str(&key_name(self.code));
        name
    }

    #[cfg(test)]
    pub(crate) fn matches(self, key: KeyEvent) -> bool {
        self.code == key.code && self.modifiers == key.modifiers
    }
}

fn key_name(code: KeyCode) -> String {
    match code {
        KeyCode::Char(character) => character.to_string(),
        KeyCode::Tab => "tab".to_string(),
        KeyCode::Enter => "enter".to_string(),
        KeyCode::Esc => "esc".to_string(),
        KeyCode::Backspace => "backspace".to_string(),
        KeyCode::Up => "up".to_string(),
        KeyCode::Down => "down".to_string(),
        KeyCode::Left => "left".to_string(),
        KeyCode::Right => "right".to_string(),
        KeyCode::PageUp => "pageup".to_string(),
        KeyCode::PageDown => "pagedown".to_string(),
        KeyCode::Home => "home".to_string(),
        KeyCode::End => "end".to_string(),
        _ => String::new(),
    }
}

fn key_code(name: &str) -> Option<KeyCode> {
    let mut chars = name.chars();
    let first = chars.next()?;
    if chars.next().is_none() {
        return Some(KeyCode::Char(first));
    }
    match name {
        "space" => Some(KeyCode::Char(' ')),
        "tab" => Some(KeyCode::Tab),
        "enter" | "return" => Some(KeyCode::Enter),
        "esc" | "escape" => Some(KeyCode::Esc),
        "backspace" => Some(KeyCode::Backspace),
        "up" => Some(KeyCode::Up),
        "down" => Some(KeyCode::Down),
        "left" => Some(KeyCode::Left),
        "right" => Some(KeyCode::Right),
        "pageup" => Some(KeyCode::PageUp),
        "pagedown" => Some(KeyCode::PageDown),
        "home" => Some(KeyCode::Home),
        "end" => Some(KeyCode::End),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_control_bindings() {
        let binding = KeyBinding::parse("ctrl-b").expect("binding");
        assert!(binding.matches(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL)));
        assert!(!binding.matches(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE)));
    }

    #[test]
    fn parses_named_keys_and_multiple_modifiers() {
        let binding = KeyBinding::parse("ctrl-shift-pageup");
        assert_eq!(
            binding,
            Some(KeyBinding {
                code: KeyCode::PageUp,
                modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            })
        );
        assert_eq!(
            binding.map(KeyBinding::name),
            Some("ctrl-shift-pageup".to_string())
        );
    }

    #[test]
    fn rejects_empty_and_unknown_bindings() {
        assert_eq!(KeyBinding::parse(""), None);
        assert_eq!(KeyBinding::parse("ctrl-unknown"), None);
    }
}
