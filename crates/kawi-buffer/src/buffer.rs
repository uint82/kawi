use anyhow::{Context, Result};
use std::fs;

pub struct Buffer {
    lines: Vec<String>,
    pub path: Option<String>,
}

impl Buffer {
    pub fn from_string(s: &str) -> Self {
        Self::from_lines(split_lines(s), None)
    }

    pub fn from_file(path: &str) -> Result<Self> {
        let text = read_file_text(path)?;
        Ok(Self::from_lines(split_lines(&text), Some(path.to_string())))
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

    pub fn insert_char(&mut self, row: usize, col: usize, ch: char) {
        self.ensure_line_exists(row);
        let byte_idx = char_to_byte_index(&self.lines[row], col);
        self.lines[row].insert(byte_idx, ch);
    }

    pub fn insert_newline(&mut self, row: usize, col: usize) {
        self.ensure_line_exists(row);
        let byte_idx = char_to_byte_index(&self.lines[row], col);
        let right = self.lines[row].split_off(byte_idx);
        self.lines.insert(row + 1, right);
    }

    pub fn delete_backward(&mut self, row: usize, col: usize) -> (usize, usize) {
        if row >= self.lines.len() {
            return (row, col);
        }
        if col > 0 {
            self.remove_char_before(row, col);
            return (row, col.saturating_sub(1));
        }
        self.join_with_previous(row)
    }

    pub fn save(&self) -> Result<()> {
        let path = match &self.path {
            Some(p) => p,
            None => return Ok(()),
        };
        write_to_file(path, &self.lines)
    }

    fn from_lines(mut lines: Vec<String>, path: Option<String>) -> Self {
        if lines.is_empty() {
            lines.push(String::new());
        }
        Self { lines, path }
    }

    fn ensure_line_exists(&mut self, row: usize) {
        while self.lines.len() <= row {
            self.lines.push(String::new());
        }
    }

    fn remove_char_before(&mut self, row: usize, col: usize) {
        let byte_idx = char_to_byte_index(&self.lines[row], col);
        let prev_byte_idx = char_to_byte_index(&self.lines[row], col.saturating_sub(1));
        if prev_byte_idx < byte_idx {
            self.lines[row].drain(prev_byte_idx..byte_idx);
        }
    }

    fn join_with_previous(&mut self, row: usize) -> (usize, usize) {
        if row == 0 {
            return (0, 0);
        }
        let current = self.lines.remove(row);
        let prev_row = row - 1;
        let new_col = self.lines[prev_row].chars().count();
        self.lines[prev_row].push_str(&current);
        (prev_row, new_col)
    }
}

fn split_lines(s: &str) -> Vec<String> {
    s.lines().map(str::to_string).collect()
}

fn read_file_text(path: &str) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("failed to open {path}"))
}

fn char_to_byte_index(text: &str, col: usize) -> usize {
    text.char_indices()
        .nth(col)
        .map_or(text.len(), |(idx, _)| idx)
}

fn write_to_file(path: &str, lines: &[String]) -> Result<()> {
    let text = lines.join("\n");
    fs::write(path, text).with_context(|| format!("failed to save {path}"))
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
    fn empty_has_one_line() {
        let b = Buffer::from_string("");
        assert_eq!(b.len(), 1);
        assert_eq!(b.line(0), Some(""));
    }

    #[test]
    fn insert_char_places_char() {
        let mut b = Buffer::from_string("ac");
        b.insert_char(0, 1, 'b');
        assert_eq!(b.line(0), Some("abc"));
    }

    #[test]
    fn insert_newline_splits_line() {
        let mut b = Buffer::from_string("ab");
        b.insert_newline(0, 1);
        assert_eq!(b.len(), 2);
        assert_eq!(b.line(0), Some("a"));
        assert_eq!(b.line(1), Some("b"));
    }

    #[test]
    fn delete_backward_removes_char() {
        let mut b = Buffer::from_string("ab");
        let pos = b.delete_backward(0, 2);
        assert_eq!(b.line(0), Some("a"));
        assert_eq!(pos, (0, 1));
    }

    #[test]
    fn delete_backward_joins_lines() {
        let mut b = Buffer::from_string("a\nb");
        let pos = b.delete_backward(1, 0);
        assert_eq!(b.len(), 1);
        assert_eq!(b.line(0), Some("ab"));
        assert_eq!(pos, (0, 1));
    }
}
