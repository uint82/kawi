use anyhow::{Context, Result};
use std::fs;

use crate::history::History;

pub struct Buffer {
    lines: Vec<String>,
    pub path: Option<String>,
    history: History,
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
        self.history.is_dirty()
    }

    pub fn begin_change(&mut self) {
        self.history.begin_change();
    }

    pub fn end_change(&mut self) {
        self.history.end_change();
    }

    pub fn checkpoint(&mut self, row: usize, col: usize) {
        self.history.checkpoint(&self.lines, row, col);
    }

    pub fn maybe_checkpoint(&mut self, row: usize, col: usize) {
        self.history.maybe_checkpoint(&self.lines, row, col);
    }

    pub fn undo(&mut self, row: usize, col: usize) -> Option<(usize, usize)> {
        self.history.undo(&mut self.lines, row, col)
    }

    pub fn redo(&mut self, row: usize, col: usize) -> Option<(usize, usize)> {
        self.history.redo(&mut self.lines, row, col)
    }

    pub fn redo_keep_cursor(&mut self, row: usize, col: usize) {
        let _ = self.history.redo(&mut self.lines, row, col);
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
        self.delete_at_line_start(row)
    }

    pub fn delete_forward(&mut self, row: usize, col: usize) {
        if row >= self.lines.len() {
            return;
        }
        if col < self.lines[row].chars().count() {
            self.remove_char_at(row, col);
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
        }
    }

    pub fn line_text(&self, row: usize) -> String {
        self.lines.get(row).cloned().unwrap_or_default()
    }

    pub fn clear_line(&mut self, row: usize) {
        if let Some(line) = self.lines.get_mut(row) {
            line.clear();
        }
    }

    pub fn paste_lines_below(&mut self, row: usize, lines: &[String]) -> usize {
        let insert_at = row.saturating_add(1).min(self.lines.len());
        for (offset, line) in lines.iter().enumerate() {
            self.lines
                .insert(insert_at.saturating_add(offset), line.clone());
        }
        insert_at
    }

    pub fn range_text(
        &self,
        start_row: usize,
        start_col: usize,
        end_row: usize,
        end_col: usize,
    ) -> String {
        if start_row == end_row {
            return single_line_slice(&self.lines, start_row, start_col, end_col);
        }
        multiline_slice(&self.lines, start_row, start_col, end_row, end_col)
    }

    pub fn delete_range(
        &mut self,
        start_row: usize,
        start_col: usize,
        end_row: usize,
        end_col: usize,
    ) {
        if start_row >= self.lines.len() {
            return;
        }
        let end_row = end_row.min(self.lines.len().saturating_sub(1));
        if start_row == end_row {
            delete_in_line(&mut self.lines, start_row, start_col, end_col);
            return;
        }
        delete_across_lines(&mut self.lines, start_row, start_col, end_row, end_col);
    }

    pub fn paste_after(&mut self, row: usize, col: usize, text: &str) -> (usize, usize) {
        let fragments: Vec<&str> = text.split('\n').collect();
        match fragments.as_slice() {
            [first] => self.paste_single(row, col, first),
            [first, rest @ ..] => self.paste_multiple(row, col, first, rest),
            [] => (row, col),
        }
    }

    fn paste_single(&mut self, row: usize, col: usize, fragment: &str) -> (usize, usize) {
        self.ensure_line_exists(row);
        let byte_idx = insert_byte_index(&self.lines[row], col);
        self.lines[row].insert_str(byte_idx, fragment);
        (row, col.saturating_add(char_count(fragment)))
    }

    fn paste_multiple(
        &mut self,
        row: usize,
        col: usize,
        first: &str,
        rest: &[&str],
    ) -> (usize, usize) {
        self.ensure_line_exists(row);
        let (left, right) = split_line_at(&self.lines[row], col);
        let last = rest.last().copied().unwrap_or("");
        self.lines[row] = format!("{left}{first}");
        let mut insert_at = row.saturating_add(1);
        for middle in rest.iter().take(rest.len().saturating_sub(1)) {
            self.lines.insert(insert_at, (*middle).to_string());
            insert_at = insert_at.saturating_add(1);
        }
        self.lines.insert(insert_at, format!("{last}{right}"));
        (insert_at, col.saturating_add(char_count(last)))
    }

    pub fn save(&mut self) -> Result<()> {
        let path = match &self.path {
            Some(p) => p.clone(),
            None => return Ok(()),
        };
        write_to_file(&path, &self.lines)?;
        self.history.mark_clean();
        Ok(())
    }

    fn from_lines(mut lines: Vec<String>, path: Option<String>) -> Self {
        if lines.is_empty() {
            lines.push(String::new());
        }
        Self {
            lines,
            path,
            history: History::new(),
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
    }

    fn clear_single_line(&mut self) {
        if self.lines.is_empty() {
            self.lines.push(String::new());
            return;
        }
        if !self.lines[0].is_empty() {
            self.lines[0].clear();
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
        self.join_with_previous(row)
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

fn char_count(text: &str) -> usize {
    text.chars().count()
}

pub fn first_non_blank(line: &str) -> usize {
    line.chars()
        .enumerate()
        .find(|(_, ch)| !ch.is_whitespace())
        .map_or(0, |(idx, _)| idx)
}

fn insert_byte_index(line: &str, col: usize) -> usize {
    char_to_byte_index(line, col.saturating_add(1))
}

fn split_line_at(line: &str, col: usize) -> (String, String) {
    let byte_idx = insert_byte_index(line, col);
    (line[..byte_idx].to_string(), line[byte_idx..].to_string())
}

fn single_line_slice(lines: &[String], row: usize, start_col: usize, end_col: usize) -> String {
    match lines.get(row) {
        Some(line) => slice_cols(line, start_col, end_col.saturating_add(1)),
        None => String::new(),
    }
}

fn multiline_slice(
    lines: &[String],
    start_row: usize,
    start_col: usize,
    end_row: usize,
    end_col: usize,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    for row in start_row..=end_row {
        parts.push(slice_row(
            lines, row, start_row, start_col, end_row, end_col,
        ));
    }
    parts.join("\n")
}

fn slice_row(
    lines: &[String],
    row: usize,
    start_row: usize,
    start_col: usize,
    end_row: usize,
    end_col: usize,
) -> String {
    let line = lines.get(row).map_or("", String::as_str);
    let from = slice_from(row, start_row, start_col);
    let to = slice_to(row, line, end_row, end_col);
    slice_cols(line, from, to)
}

fn slice_from(row: usize, start_row: usize, start_col: usize) -> usize {
    if row == start_row {
        start_col
    } else {
        0
    }
}

fn slice_to(row: usize, line: &str, end_row: usize, end_col: usize) -> usize {
    if row == end_row {
        end_col.saturating_add(1)
    } else {
        char_count(line)
    }
}

fn slice_cols(text: &str, from: usize, to: usize) -> String {
    text.chars()
        .skip(from)
        .take(to.saturating_sub(from))
        .collect()
}

fn delete_in_line(lines: &mut [String], row: usize, start_col: usize, end_col: usize) {
    if let Some(line) = lines.get_mut(row) {
        let start = char_to_byte_index(line, start_col);
        let end = char_to_byte_index(line, end_col.saturating_add(1));
        if start < end {
            line.drain(start..end);
        }
    }
}

fn delete_across_lines(
    lines: &mut Vec<String>,
    start_row: usize,
    start_col: usize,
    end_row: usize,
    end_col: usize,
) {
    if let Some(joined) = join_range(lines, start_row, start_col, end_row, end_col) {
        lines.splice(start_row..=end_row, [joined]);
    }
}

fn join_range(
    lines: &[String],
    start_row: usize,
    start_col: usize,
    end_row: usize,
    end_col: usize,
) -> Option<String> {
    let first = lines.get(start_row)?;
    let last = lines.get(end_row)?;
    let head = slice_cols(first, 0, start_col);
    let tail = slice_cols(last, end_col.saturating_add(1), char_count(last));
    Some(format!("{head}{tail}"))
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
        b.checkpoint(0, 0);
        b.insert_char(0, 1, 'b');
        assert!(b.is_dirty());
    }

    #[test]
    fn save_clears_dirty() {
        let path = std::env::temp_dir().join("kawi_dirty_test.txt");
        let path_str = path.to_string_lossy().to_string();
        let mut b = Buffer::from_string("hi");
        b.path = Some(path_str);
        b.checkpoint(0, 0);
        b.insert_char(0, 2, '!');
        assert!(b.is_dirty());
        b.save().expect("save temp file");
        assert!(!b.is_dirty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn undo_restores_lines() {
        let mut b = Buffer::from_string("ab");
        b.checkpoint(0, 0);
        b.insert_char(0, 2, 'c');
        assert!(b.undo(0, 0).is_some());
        assert_eq!(b.line(0), Some("ab"));
    }

    #[test]
    fn redo_restores_forward() {
        let mut b = Buffer::from_string("ab");
        b.checkpoint(0, 0);
        b.insert_char(0, 2, 'c');
        b.undo(0, 0);
        assert!(b.redo(0, 0).is_some());
        assert_eq!(b.line(0), Some("abc"));
    }

    #[test]
    fn undo_without_history_returns_false() {
        let mut b = Buffer::from_string("ab");
        assert_eq!(b.undo(0, 0), None);
        assert_eq!(b.redo(0, 0), None);
    }

    #[test]
    fn delete_forward_removes_char() {
        let mut b = Buffer::from_string("ab");
        b.checkpoint(0, 0);
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
        b.checkpoint(0, 0);
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

    #[test]
    fn range_text_reads_single_line() {
        let b = Buffer::from_string("abcdef");
        assert_eq!(b.range_text(0, 1, 0, 3), "bcd");
    }

    #[test]
    fn range_text_joins_multiple_lines() {
        let b = Buffer::from_string("ab\ncd\nef");
        assert_eq!(b.range_text(0, 1, 2, 1), "b\ncd\nef");
    }

    #[test]
    fn delete_range_removes_single_line() {
        let mut b = Buffer::from_string("abcdef");
        b.delete_range(0, 1, 0, 3);
        assert_eq!(b.line(0), Some("aef"));
    }

    #[test]
    fn delete_range_joins_remainders() {
        let mut b = Buffer::from_string("ab\ncd\nef");
        b.delete_range(0, 1, 2, 1);
        assert_eq!(b.len(), 1);
        assert_eq!(b.line(0), Some("a"));
    }

    #[test]
    fn paste_after_inserts_single_line() {
        let mut b = Buffer::from_string("aef");
        assert_eq!(b.paste_after(0, 1, "bcd"), (0, 4));
        assert_eq!(b.line(0), Some("aebcdf"));
    }

    #[test]
    fn paste_after_splits_multiple_lines() {
        let mut b = Buffer::from_string("12");
        assert_eq!(b.paste_after(0, 0, "ab\nc"), (1, 1));
        assert_eq!(b.len(), 2);
        assert_eq!(b.line(0), Some("1ab"));
        assert_eq!(b.line(1), Some("c2"));
    }

    #[test]
    fn paste_empty_changes_nothing() {
        let mut b = Buffer::from_string("ab");
        assert_eq!(b.paste_after(0, 0, ""), (0, 0));
        assert_eq!(b.line(0), Some("ab"));
    }

    #[test]
    fn paste_lines_below_inserts() {
        let mut b = Buffer::from_string("aa\nbb\ncc");
        assert_eq!(b.paste_lines_below(1, &["bb".to_string()]), 2);
        assert_eq!(b.len(), 4);
        assert_eq!(b.line(2), Some("bb"));
    }

    #[test]
    fn clear_line_empties() {
        let mut b = Buffer::from_string("abcdef");
        b.clear_line(0);
        assert_eq!(b.line(0), Some(""));
        assert_eq!(b.len(), 1);
    }

    #[test]
    fn first_non_blank_skips_space() {
        assert_eq!(first_non_blank("  ind"), 2);
        assert_eq!(first_non_blank("ab"), 0);
        assert_eq!(first_non_blank(""), 0);
    }
}
