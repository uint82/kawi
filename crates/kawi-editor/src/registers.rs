use crate::register::{Register, RegisterKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RegisterSlot {
    Numbered(usize),
    Named(usize),
    Small,
    Star,
    Plus,
}

pub(crate) struct Registers {
    numbered: [Option<Register>; 10],
    named: [Option<Register>; 26],
    small: Option<Register>,
    star: Option<Register>,
    plus: Option<Register>,
    unnamed: Option<RegisterSlot>,
}

impl Registers {
    pub(crate) fn new() -> Self {
        Self {
            numbered: Default::default(),
            named: Default::default(),
            small: None,
            star: None,
            plus: None,
            unnamed: None,
        }
    }

    pub(crate) fn unnamed_slot(&self) -> Option<RegisterSlot> {
        self.unnamed
    }

    pub(crate) fn get(&self, slot: RegisterSlot) -> Option<&Register> {
        match slot {
            RegisterSlot::Numbered(index) => self.numbered.get(index).and_then(Option::as_ref),
            RegisterSlot::Named(index) => self.named.get(index).and_then(Option::as_ref),
            RegisterSlot::Small => self.small.as_ref(),
            RegisterSlot::Star => self.star.as_ref(),
            RegisterSlot::Plus => self.plus.as_ref(),
        }
    }

    pub(crate) fn yank(&mut self, name: Option<char>, register: Register) -> RegisterSlot {
        let slot = self.yank_slot(name);
        self.write(slot, register, is_append(name));
        self.unnamed = Some(slot);
        slot
    }

    pub(crate) fn delete(
        &mut self,
        name: Option<char>,
        register: Register,
        line_count: usize,
        linewise: bool,
        numbered: bool,
    ) -> Option<RegisterSlot> {
        if name == Some('_') {
            return None;
        }
        let mut named = None;
        if let Some(name) = name {
            let slot = match name {
                '"' => Some(RegisterSlot::Numbered(0)),
                other => self.slot_of(other),
            };
            if let Some(slot) = slot {
                self.write(slot, register.clone(), name.is_ascii_uppercase());
                self.unnamed = Some(slot);
                named = Some(slot);
            }
        }
        if linewise || line_count > 1 || numbered {
            self.shift(is_append(name));
            self.write(RegisterSlot::Numbered(1), register.clone(), false);
        }
        if name.is_none() && !linewise && line_count == 1 {
            self.write(RegisterSlot::Small, register, false);
            self.unnamed = Some(RegisterSlot::Small);
        }
        named
    }

    pub(crate) fn slot(&self, name: char) -> Option<RegisterSlot> {
        if name == '_' || name == '+' || name == '*' {
            return None;
        }
        self.slot_of(name)
    }

    pub(crate) fn store_clipboard(&mut self, slot: RegisterSlot, register: Register) {
        match slot {
            RegisterSlot::Star => self.star = Some(register),
            RegisterSlot::Plus => self.plus = Some(register),
            _ => {}
        }
    }

    fn yank_slot(&self, name: Option<char>) -> RegisterSlot {
        match name {
            None | Some('"') => RegisterSlot::Numbered(0),
            Some(other) => self.slot_of(other).unwrap_or(RegisterSlot::Numbered(0)),
        }
    }

    fn slot_of(&self, name: char) -> Option<RegisterSlot> {
        match name {
            '0'..='9' => Some(RegisterSlot::Numbered((name as u8 - b'0') as usize)),
            'a'..='z' => Some(RegisterSlot::Named((name as u8 - b'a') as usize)),
            'A'..='Z' => Some(RegisterSlot::Named((name as u8 - b'A') as usize)),
            '-' => Some(RegisterSlot::Small),
            '*' => Some(RegisterSlot::Star),
            '+' => Some(RegisterSlot::Plus),
            _ => None,
        }
    }

    fn write(&mut self, slot: RegisterSlot, register: Register, append: bool) {
        let existing = self.get(slot);
        let merged = match existing {
            Some(existing) if append => merge(existing, &register),
            _ => register,
        };
        match slot {
            RegisterSlot::Numbered(index) => self.numbered[index] = Some(merged),
            RegisterSlot::Named(index) => self.named[index] = Some(merged),
            RegisterSlot::Small => self.small = Some(merged),
            RegisterSlot::Star => self.star = Some(merged),
            RegisterSlot::Plus => self.plus = Some(merged),
        }
    }

    fn shift(&mut self, append: bool) {
        self.numbered[9] = None;
        for index in (2..=9).rev() {
            self.numbered[index] = self.numbered[index - 1].take();
        }
        self.numbered[1] = None;
        if !append {
            self.unnamed = Some(RegisterSlot::Numbered(1));
        }
    }
}

fn is_append(name: Option<char>) -> bool {
    name.is_some_and(|name| name.is_ascii_uppercase())
}

fn merge(existing: &Register, register: &Register) -> Register {
    let kind = if register.kind() == RegisterKind::LineWise {
        RegisterKind::LineWise
    } else {
        existing.kind()
    };
    let text = match kind {
        RegisterKind::CharWise => format!("{}{}", existing.text(), register.text()),
        RegisterKind::LineWise => {
            if existing.text().is_empty() {
                register.text().to_string()
            } else {
                format!("{}\n{}", existing.text(), register.text())
            }
        }
    };
    Register::from_parts(text, kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn register(kind: RegisterKind, text: &str) -> Register {
        Register::from_parts(text.to_string(), kind)
    }

    fn text(registers: &Registers, slot: RegisterSlot) -> Option<(RegisterKind, String)> {
        registers
            .get(slot)
            .map(|register| (register.kind(), register.text().to_string()))
    }

    #[test]
    fn unnamed_yank_writes_register_zero() {
        let mut registers = Registers::new();
        registers.yank(None, register(RegisterKind::LineWise, "a"));
        assert_eq!(
            text(&registers, RegisterSlot::Numbered(0)),
            Some((RegisterKind::LineWise, "a".to_string()))
        );
        assert_eq!(registers.unnamed_slot(), Some(RegisterSlot::Numbered(0)));
    }

    #[test]
    fn named_yank_leaves_register_zero() {
        let mut registers = Registers::new();
        registers.yank(Some('a'), register(RegisterKind::CharWise, "x"));
        assert_eq!(text(&registers, RegisterSlot::Numbered(0)), None);
        assert_eq!(registers.unnamed_slot(), Some(RegisterSlot::Named(0)));
    }

    #[test]
    fn uppercase_yank_appends() {
        let mut registers = Registers::new();
        registers.yank(Some('a'), register(RegisterKind::CharWise, "ab"));
        registers.yank(Some('A'), register(RegisterKind::CharWise, "cd"));
        assert_eq!(
            text(&registers, RegisterSlot::Named(0)),
            Some((RegisterKind::CharWise, "abcd".to_string()))
        );
    }

    #[test]
    fn uppercase_linewise_append_adds_line() {
        let mut registers = Registers::new();
        registers.yank(Some('a'), register(RegisterKind::CharWise, "ab"));
        registers.yank(Some('A'), register(RegisterKind::LineWise, "cd"));
        assert_eq!(
            text(&registers, RegisterSlot::Named(0)),
            Some((RegisterKind::LineWise, "ab\ncd".to_string()))
        );
    }

    #[test]
    fn linewise_delete_shifts_numbered_registers() {
        let mut registers = Registers::new();
        registers.delete(None, register(RegisterKind::LineWise, "a"), 1, true, false);
        registers.delete(None, register(RegisterKind::LineWise, "b"), 1, true, false);
        assert_eq!(
            text(&registers, RegisterSlot::Numbered(1)),
            Some((RegisterKind::LineWise, "b".to_string()))
        );
        assert_eq!(
            text(&registers, RegisterSlot::Numbered(2)),
            Some((RegisterKind::LineWise, "a".to_string()))
        );
        assert_eq!(registers.unnamed_slot(), Some(RegisterSlot::Numbered(1)));
    }

    #[test]
    fn small_delete_fills_small_register() {
        let mut registers = Registers::new();
        registers.delete(None, register(RegisterKind::CharWise, "a"), 1, false, false);
        assert_eq!(
            text(&registers, RegisterSlot::Small),
            Some((RegisterKind::CharWise, "a".to_string()))
        );
        assert_eq!(text(&registers, RegisterSlot::Numbered(1)), None);
        assert_eq!(registers.unnamed_slot(), Some(RegisterSlot::Small));
    }

    #[test]
    fn numbered_delete_uses_register_one() {
        let mut registers = Registers::new();
        registers.delete(
            Some('a'),
            register(RegisterKind::CharWise, "word"),
            2,
            false,
            true,
        );
        assert_eq!(
            text(&registers, RegisterSlot::Named(0)),
            Some((RegisterKind::CharWise, "word".to_string()))
        );
        assert_eq!(
            text(&registers, RegisterSlot::Numbered(1)),
            Some((RegisterKind::CharWise, "word".to_string()))
        );
        assert_eq!(registers.unnamed_slot(), Some(RegisterSlot::Numbered(1)));
    }

    #[test]
    fn named_small_delete_skips_delete_registers() {
        let mut registers = Registers::new();
        registers.delete(
            Some('a'),
            register(RegisterKind::CharWise, "x"),
            1,
            false,
            false,
        );
        assert_eq!(
            text(&registers, RegisterSlot::Named(0)),
            Some((RegisterKind::CharWise, "x".to_string()))
        );
        assert_eq!(text(&registers, RegisterSlot::Numbered(1)), None);
        assert_eq!(text(&registers, RegisterSlot::Small), None);
        assert_eq!(registers.unnamed_slot(), Some(RegisterSlot::Named(0)));
    }

    #[test]
    fn append_delete_keeps_named_unnamed() {
        let mut registers = Registers::new();
        registers.delete(
            Some('a'),
            register(RegisterKind::LineWise, "a"),
            1,
            true,
            false,
        );
        registers.delete(
            Some('A'),
            register(RegisterKind::LineWise, "b"),
            1,
            true,
            false,
        );
        assert_eq!(
            text(&registers, RegisterSlot::Named(0)),
            Some((RegisterKind::LineWise, "a\nb".to_string()))
        );
        assert_eq!(registers.unnamed_slot(), Some(RegisterSlot::Named(0)));
    }

    #[test]
    fn black_hole_delete_writes_nothing() {
        let mut registers = Registers::new();
        registers.yank(None, register(RegisterKind::LineWise, "a"));
        registers.delete(
            Some('_'),
            register(RegisterKind::LineWise, "b"),
            1,
            true,
            false,
        );
        assert_eq!(
            text(&registers, RegisterSlot::Numbered(0)),
            Some((RegisterKind::LineWise, "a".to_string()))
        );
        assert_eq!(registers.unnamed_slot(), Some(RegisterSlot::Numbered(0)));
    }

    #[test]
    fn explicit_unnamed_yank_writes_register_zero() {
        let mut registers = Registers::new();
        registers.yank(Some('"'), register(RegisterKind::CharWise, "x"));
        assert_eq!(
            text(&registers, RegisterSlot::Numbered(0)),
            Some((RegisterKind::CharWise, "x".to_string()))
        );
        assert_eq!(registers.unnamed_slot(), Some(RegisterSlot::Numbered(0)));
    }
}
