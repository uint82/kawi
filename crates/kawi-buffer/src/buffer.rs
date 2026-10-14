use anyhow::{Context, Result};
use std::fs;

pub struct Buffer {
    lines: Vec<String>,
    pub path: Option<String>,
    dirty: bool,
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

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn insert_char(&mut self, row: usize, col: usize, ch: char) {
        self.ensure_line_exists(row);
        let byte_idx = char_to_byte_index(&self.lines[row], col);
        self.lines[row].insert(byte_idx, ch);
        self.mark_dirty();
    }

    pub fn insert_newline(&mut self, row: usize, col: usize) {
        self.ensure_line_exists(row);
        let byte_idx = char_to_byte_index(&self.lines[row], col);
        let right = self.lines[row].split_off(byte_idx);
        self.lines.insert(row + 1, right);
        self.mark_dirty();
    }

    pub fn delete_backward(&mut self, row: usize, col: usize) -> (usize, usize) {
        if row >= self.lines.len() {
            return (row, col);
        }
        if col > 0 {
            self.remove_char_before(row, col);
            self.mark_dirty();
            return (row, col.saturating_sub(1));
        }
        self.delete_at_line_start(row)
    }

    pub fn delete_forward(&mut self, row: usize, col: usize) {
        if row >= self.lines.len() {
            return;
        }
        if col < self.lines[row].chars().count() {
            self.remove_char_at(row, col);
            self.mark_dirty();
            return;
        }
        self.delete_at_line_end(row);
    }

    pub fn delete_line(&mut self, row: usize) {
        if self.lines.len() <= 1 {
            self.clear_single_line();
            return;
        }
        if row < self.lines.len() {
            self.lines.remove(row);
            self.mark_dirty();
        }
    }

    pub fn save(&mut self) -> Result<()> {
        let path = match &self.path {
            Some(p) => p.clone(),
            None => return Ok(()),
        };
        write_to_file(&path, &self.lines)?;
        self.mark_clean();
        Ok(())
    }

    fn from_lines(mut lines: Vec<String>, path: Option<String>) -> Self {
        if lines.is_empty() {
            lines.push(String::new());
        }
        Self {
            lines,
            path,
            dirty: false,
        }
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

    fn remove_char_at(&mut self, row: usize, col: usize) {
        let start = char_to_byte_index(&self.lines[row], col);
        let end = char_to_byte_index(&self.lines[row], col.saturating_add(1));
        if start < end {
            self.lines[row].drain(start..end);
        }
    }

    fn delete_at_line_end(&mut self, row: usize) {
        if row.saturating_add(1) >= self.lines.len() {
            return;
        }
        let next = self.lines.remove(row.saturating_add(1));
        self.lines[row].push_str(&next);
        self.mark_dirty();
    }

    fn clear_single_line(&mut self) {
        if self.lines.is_empty() {
            self.lines.push(String::new());
            return;
        }
        if !self.lines[0].is_empty() {
            self.lines[0].clear();
            self.mark_dirty();
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

    fn delete_at_line_start(&mut self, row: usize) -> (usize, usize) {
        if row == 0 {
            return (0, 0);
        }
        let pos = self.join_with_previous(row);
        self.mark_dirty();
        pos
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
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

    #[test]
    fn fresh_buffer_is_clean() {
        let b = Buffer::from_string("a");
        assert!(!b.is_dirty());
    }

    #[test]
    fn edit_marks_dirty() {
        let mut b = Buffer::from_string("a");
        b.insert_char(0, 1, 'b');
        assert!(b.is_dirty());
    }

    #[test]
    fn save_clears_dirty() {
        let path = std::env::temp_dir().join("kawi_dirty_test.txt");
        let path_str = path.to_string_lossy().to_string();
        let mut b = Buffer::from_string("hi");
        b.path = Some(path_str);
        b.insert_char(0, 2, '!');
        assert!(b.is_dirty());
        b.save().expect("save temp file");
        assert!(!b.is_dirty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn delete_forward_removes_char() {
        let mut b = Buffer::from_string("ab");
        b.delete_forward(0, 0);
        assert_eq!(b.line(0), Some("b"));
        assert!(b.is_dirty());
    }

    #[test]
    fn delete_forward_joins_lines() {
        let mut b = Buffer::from_string("a\nb");
        b.delete_forward(0, 1);
        assert_eq!(b.len(), 1);
        assert_eq!(b.line(0), Some("ab"));
    }

    #[test]
    fn delete_line_removes_row() {
        let mut b = Buffer::from_string("a\nb\nc");
        b.delete_line(1);
        assert_eq!(b.len(), 2);
        assert_eq!(b.line(1), Some("c"));
        assert!(b.is_dirty());
    }

    #[test]
    fn delete_last_line_keeps_one_empty() {
        let mut b = Buffer::from_string("a");
        b.delete_line(0);
        assert_eq!(b.len(), 1);
        assert_eq!(b.line(0), Some(""));
    }
}
