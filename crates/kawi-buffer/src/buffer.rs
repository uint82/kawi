use anyhow::{Context, Result};
use std::fs;

pub struct Buffer {
    lines: Vec<String>,
    pub path: Option<String>,
}

impl Buffer {
    pub fn from_string(s: &str) -> Self {
        Self {
            lines: split_lines(s),
            path: None,
        }
    }

    pub fn from_file(path: &str) -> Result<Self> {
        let text = read_file_text(path)?;
        Ok(Self {
            lines: split_lines(&text),
            path: Some(path.to_string()),
        })
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn line(&self, i: usize) -> Option<&str> {
        self.lines.get(i).map(String::as_str)
    }

    pub fn as_slice(&self) -> &[String] {
        &self.lines
    }
}

fn split_lines(s: &str) -> Vec<String> {
    s.lines().map(str::to_string).collect()
}

fn read_file_text(path: &str) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("failed to open {path}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_string_splits_lines() {
        let b = Buffer::from_string("a\nb");
        assert_eq!(b.len(), 2);
        assert_eq!(b.line(0), Some("a"));
    }

    #[test]
    fn empty_stays_empty() {
        let b = Buffer::from_string("");
        assert!(b.is_empty());
    }
}
