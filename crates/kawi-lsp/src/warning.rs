use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageWarning {
    line: Option<usize>,
    message: String,
}

impl LanguageWarning {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            line: None,
            message: message.into(),
        }
    }

    pub fn at_line(mut self, line: Option<usize>) -> Self {
        self.line = line;
        self
    }

    pub fn line(&self) -> Option<usize> {
        self.line
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn format(&self, path: &Path) -> String {
        match self.line {
            Some(line) => format!("{}:{line}: {}", path.display(), self.message),
            None => format!("{}: {}", path.display(), self.message),
        }
    }
}

impl fmt::Display for LanguageWarning {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(formatter, "line {line}: {}", self.message),
            None => formatter.write_str(&self.message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_with_and_without_a_line() {
        let path = Path::new("languages.toml");
        assert_eq!(
            LanguageWarning::new("bad value")
                .at_line(Some(4))
                .format(path),
            "languages.toml:4: bad value"
        );
        assert_eq!(
            LanguageWarning::new("bad value").format(path),
            "languages.toml: bad value"
        );
    }
}
