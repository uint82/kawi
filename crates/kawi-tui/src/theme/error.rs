use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeError {
    line: Option<usize>,
    message: String,
    not_found: bool,
}

impl ThemeError {
    pub(super) fn invalid(message: impl Into<String>) -> Self {
        Self {
            line: None,
            message: message.into(),
            not_found: false,
        }
    }

    pub(super) fn not_found(name: &str) -> Self {
        Self {
            line: None,
            message: format!("theme \"{name}\" not found"),
            not_found: true,
        }
    }

    pub(super) fn at_line(mut self, line: Option<usize>) -> Self {
        self.line = line;
        self
    }

    pub fn is_not_found(&self) -> bool {
        self.not_found
    }
}

impl fmt::Display for ThemeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(formatter, "line {line}: {}", self.message),
            None => formatter.write_str(&self.message),
        }
    }
}

impl std::error::Error for ThemeError {}
