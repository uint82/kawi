#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterKind {
    CharWise,
    LineWise,
}

pub struct Register {
    text: String,
    kind: RegisterKind,
}

impl Register {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            kind: RegisterKind::CharWise,
        }
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.kind = RegisterKind::CharWise;
    }

    pub fn set_lines(&mut self, text: String) {
        self.text = text;
        self.kind = RegisterKind::LineWise;
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn kind(&self) -> RegisterKind {
        self.kind
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

impl Default for Register {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_empty_charwise() {
        let register = Register::new();
        assert!(register.is_empty());
        assert_eq!(register.kind(), RegisterKind::CharWise);
    }

    #[test]
    fn text_sets_charwise() {
        let mut register = Register::new();
        register.set_lines("a".to_string());
        register.set_text("bcd".to_string());
        assert_eq!(register.text(), "bcd");
        assert_eq!(register.kind(), RegisterKind::CharWise);
    }

    #[test]
    fn lines_sets_linewise() {
        let mut register = Register::new();
        register.set_lines("a\nb".to_string());
        assert_eq!(register.text(), "a\nb");
        assert_eq!(register.kind(), RegisterKind::LineWise);
    }
}
