#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterKind {
    CharWise,
    LineWise,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

    pub(crate) fn from_parts(text: String, kind: RegisterKind) -> Self {
        Self { text, kind }
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

    pub fn lines(&self) -> Vec<String> {
        split_lines(&self.text)
    }
}

pub(crate) fn split_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    if text.ends_with('\n') {
        lines.push(String::new());
    }
    lines
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
    fn parts_set_text_and_kind() {
        let register = Register::from_parts("bcd".to_string(), RegisterKind::CharWise);
        assert_eq!(register.text(), "bcd");
        assert_eq!(register.kind(), RegisterKind::CharWise);
    }

    #[test]
    fn lines_keeps_trailing_blank_line() {
        let register = Register::from_parts("a\nb\n".to_string(), RegisterKind::LineWise);
        assert_eq!(
            register.lines(),
            vec!["a".to_string(), "b".to_string(), String::new()]
        );
    }

    #[test]
    fn lines_splits_without_trailing_newline() {
        let register = Register::from_parts("a\nb".to_string(), RegisterKind::LineWise);
        assert_eq!(register.lines(), vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn lines_empty_text_is_one_blank_line() {
        let register = Register::from_parts(String::new(), RegisterKind::LineWise);
        assert_eq!(register.lines(), vec![String::new()]);
    }

    #[test]
    fn lines_sets_linewise() {
        let register = Register::from_parts("a\nb".to_string(), RegisterKind::LineWise);
        assert_eq!(register.text(), "a\nb");
        assert_eq!(register.kind(), RegisterKind::LineWise);
    }
}
