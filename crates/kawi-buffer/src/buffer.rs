use anyhow::{Context, Result};
use ropey::Rope;
use std::fs;

use crate::history::History;

pub struct Buffer {
    rope: Rope,
    pub path: Option<String>,
    history: History,
    revision: u64,
}

impl Buffer {
    pub fn from_string(s: &str) -> Self {
        Self {
            rope: Rope::from_str(s),
            path: None,
            history: History::new(),
            revision: 0,
        }
    }

    pub fn from_file(path: &str) -> Result<Self> {
        let text = read_file_text(path)?;
        Ok(Self {
            rope: Rope::from_str(&text),
            path: Some(path.to_string()),
            history: History::new(),
            revision: 0,
        })
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    fn touch(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn len(&self) -> usize {
        if self.rope.len_chars() == 0 {
            return 1;
        }
        let lines = self.rope.len_lines();
        if self.ends_with_newline() {
            lines.saturating_sub(1).max(1)
        } else {
            lines
        }
    }

    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    pub fn line(&self, row: usize) -> Option<String> {
        self.line_cow(row).map(std::borrow::Cow::into_owned)
    }

    pub fn line_cow(&self, row: usize) -> Option<std::borrow::Cow<'_, str>> {
        if self.rope.len_chars() == 0 {
            return if row == 0 {
                Some(std::borrow::Cow::Borrowed(""))
            } else {
                None
            };
        }
        if row == self.len() && self.ends_with_newline() {
            return Some(std::borrow::Cow::Borrowed(""));
        }
        if row >= self.len() {
            return None;
        }
        match self.rope.line(row).as_str() {
            Some(text) => Some(strip_ending(text)),
            None => Some(std::borrow::Cow::Owned(strip_owned(
                self.rope.line(row).to_string(),
            ))),
        }
    }

    pub fn line_len(&self, row: usize) -> usize {
        self.line_cow(row).map_or(0, |line| line.chars().count())
    }

    pub fn line_count(&self) -> usize {
        self.len()
    }

    pub fn max_cursor_row(&self) -> usize {
        if self.ends_with_newline() {
            return self.len();
        }
        self.len().saturating_sub(1)
    }

    pub fn window_lines(&self, top: usize, height: usize) -> Vec<String> {
        (top..top.saturating_add(height))
            .filter_map(|row| self.line(row))
            .collect()
    }

    pub fn line_slices(&self) -> ropey::iter::Lines<'_> {
        self.rope.lines()
    }

    pub fn rope_slice(&self) -> ropey::RopeSlice<'_> {
        self.rope.slice(..)
    }

    pub fn total_chars(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn char_index_of(&self, row: usize, col: usize) -> usize {
        self.position_to_char(row, col)
    }

    pub fn position_of_char(&self, char_index: usize) -> (usize, usize) {
        if self.rope.len_chars() == 0 {
            return (0, 0);
        }
        let idx = char_index.min(self.rope.len_chars());
        let row = self.rope.char_to_line(idx);
        let col = idx.saturating_sub(self.rope.line_to_char(row));
        (row, col)
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
        self.history.checkpoint(&self.rope, row, col);
    }

    pub fn maybe_checkpoint(&mut self, row: usize, col: usize) {
        self.history.maybe_checkpoint(&self.rope, row, col);
    }

    pub fn undo(&mut self, row: usize, col: usize) -> Option<(usize, usize)> {
        let restored = self.history.undo(&mut self.rope, row, col);
        if restored.is_some() {
            self.touch();
        }
        restored
    }

    pub fn redo(&mut self, row: usize, col: usize) -> Option<(usize, usize)> {
        let restored = self.history.redo(&mut self.rope, row, col);
        if restored.is_some() {
            self.touch();
        }
        restored
    }

    pub fn redo_keep_cursor(&mut self, row: usize, col: usize) -> bool {
        let restored = self.history.redo(&mut self.rope, row, col).is_some();
        if restored {
            self.touch();
        }
        restored
    }

    pub fn insert_char(&mut self, row: usize, col: usize, ch: char) {
        self.ensure_line_exists(row);
        let idx = self.position_to_char(row, col);
        self.rope.insert(idx, &ch.to_string());
        self.touch();
    }

    pub fn insert_newline(&mut self, row: usize, col: usize) {
        self.ensure_line_exists(row);
        let idx = self.position_to_char(row, col);
        self.rope.insert(idx, "\n");
        self.touch();
    }

    pub fn delete_backward(&mut self, row: usize, col: usize) -> (usize, usize) {
        if row >= self.len() {
            return (row, col);
        }
        if col > 0 {
            let idx = self.position_to_char(row, col);
            self.rope.remove(idx.saturating_sub(1)..idx);
            self.touch();
            return (row, col.saturating_sub(1));
        }
        self.delete_at_line_start(row)
    }

    pub fn delete_forward(&mut self, row: usize, col: usize) {
        if row >= self.len() {
            return;
        }
        if col < self.line_len(row) {
            let idx = self.position_to_char(row, col);
            self.rope
                .remove(idx..idx.saturating_add(1).min(self.rope.len_chars()));
            self.touch();
            return;
        }
        self.delete_at_line_end(row);
    }

    pub fn delete_line(&mut self, row: usize) {
        if self.len() <= 1 {
            self.rope.remove(0..self.rope.len_chars());
            self.touch();
            return;
        }
        if row >= self.len() {
            return;
        }
        let (start, end) = self.line_span(row);
        self.rope.remove(start..end);
        self.touch();
    }

    pub fn line_text(&self, row: usize) -> String {
        self.line(row).unwrap_or_default()
    }

    pub fn clear_line(&mut self, row: usize) {
        if row >= self.len() {
            return;
        }
        let start = self.line_start_char(row);
        let end = start.saturating_add(self.line_len(row));
        if start < end {
            self.rope.remove(start..end);
            self.touch();
        }
    }

    pub fn insert_start_col(&self, row: usize) -> usize {
        match self.line(row) {
            Some(line) => start_col_of(&line),
            None => 0,
        }
    }

    pub fn paste_lines_below(&mut self, row: usize, lines: &[String]) -> usize {
        let row = row.min(self.len().saturating_sub(1));
        let at = self.line_end_char(row);
        let mut composed = String::new();
        for line in lines {
            composed.push('\n');
            composed.push_str(line);
        }
        self.rope.insert(at.min(self.rope.len_chars()), &composed);
        self.touch();
        row.saturating_add(1)
    }

    pub fn range_text(
        &self,
        start_row: usize,
        start_col: usize,
        end_row: usize,
        end_col: usize,
    ) -> String {
        let start = self.position_to_char(start_row, start_col);
        let end = self
            .position_to_char(end_row, end_col)
            .saturating_add(1)
            .min(self.rope.len_chars());
        self.rope.slice(start.min(end)..end.max(start)).to_string()
    }

    pub fn span_text(
        &self,
        start_row: usize,
        start_col: usize,
        end_row: usize,
        end_col: usize,
    ) -> String {
        let start = self.position_to_char(start_row, start_col);
        let end = self.position_to_char(end_row, end_col);
        self.rope.slice(start.min(end)..end.max(start)).to_string()
    }

    pub fn delete_range(
        &mut self,
        start_row: usize,
        start_col: usize,
        end_row: usize,
        end_col: usize,
    ) {
        if start_row >= self.len() {
            return;
        }
        let end_row = end_row.min(self.len().saturating_sub(1));
        let start = self.position_to_char(start_row, start_col);
        let end = self
            .position_to_char(end_row, end_col)
            .saturating_add(1)
            .min(self.rope.len_chars());
        if start < end {
            self.rope.remove(start..end);
            self.touch();
        }
    }

    pub fn delete_span(
        &mut self,
        start_row: usize,
        start_col: usize,
        end_row: usize,
        end_col: usize,
    ) {
        if start_row >= self.len() {
            return;
        }
        let end_row = end_row.min(self.len().saturating_sub(1));
        let start = self.position_to_char(start_row, start_col);
        let end = self.position_to_char(end_row, end_col);
        if start < end {
            self.rope.remove(start..end);
            self.touch();
        }
    }

    pub fn paste_after(&mut self, row: usize, col: usize, text: &str) -> (usize, usize) {
        if !text.is_empty() {
            self.touch();
        }
        let fragments: Vec<&str> = text.split('\n').collect();
        match fragments.as_slice() {
            [first] => self.paste_single(row, col, first),
            [first, rest @ ..] => self.paste_multiple(row, col, first, rest),
            [] => (row, col),
        }
    }

    fn paste_single(&mut self, row: usize, col: usize, fragment: &str) -> (usize, usize) {
        self.ensure_line_exists(row);
        let at = self.position_to_char(row, col.saturating_add(1));
        self.rope.insert(at.min(self.rope.len_chars()), fragment);
        (row, col.saturating_add(char_count(fragment)))
    }

    pub fn paste_before(&mut self, row: usize, col: usize, text: &str) -> (usize, usize) {
        if !text.is_empty() {
            self.touch();
        }
        let fragments: Vec<&str> = text.split('\n').collect();
        match fragments.as_slice() {
            [first] => self.paste_single_before(row, col, first),
            [first, rest @ ..] => self.paste_multiple_before(row, col, first, rest),
            [] => (row, col),
        }
    }

    fn paste_single_before(&mut self, row: usize, col: usize, fragment: &str) -> (usize, usize) {
        self.ensure_line_exists(row);
        let at = self.position_to_char(row, col);
        self.rope.insert(at.min(self.rope.len_chars()), fragment);
        (row, col.saturating_add(char_count(fragment)))
    }

    fn paste_multiple_before(
        &mut self,
        row: usize,
        col: usize,
        first: &str,
        rest: &[&str],
    ) -> (usize, usize) {
        self.ensure_line_exists(row);
        let width = self.line_len(row);
        let insert_col = col.min(width);
        let at = self.position_to_char(row, insert_col);
        let right = self
            .line(row)
            .map_or(String::new(), |line| slice_from_col(&line, insert_col));
        let last = rest.last().copied().unwrap_or("");
        let mut composed = String::from(first);
        for part in rest {
            composed.push('\n');
            composed.push_str(part);
        }
        composed.push_str(&right);
        let remove_end = at
            .saturating_add(char_count(&right))
            .min(self.rope.len_chars());
        self.rope.remove(at.min(remove_end)..remove_end);
        self.rope.insert(at.min(self.rope.len_chars()), &composed);
        (
            row.saturating_add(rest.len()),
            col.saturating_add(char_count(last)),
        )
    }

    fn paste_multiple(
        &mut self,
        row: usize,
        col: usize,
        first: &str,
        rest: &[&str],
    ) -> (usize, usize) {
        self.ensure_line_exists(row);
        let width = self.line_len(row);
        let insert_col = col.saturating_add(1).min(width.saturating_add(1));
        let at = self.position_to_char(row, insert_col);
        let right = self
            .line(row)
            .map_or(String::new(), |line| slice_from_col(&line, insert_col));
        let last = rest.last().copied().unwrap_or("");
        let mut composed = String::from(first);
        for part in rest {
            composed.push('\n');
            composed.push_str(part);
        }
        composed.push_str(&right);
        let remove_end = at
            .saturating_add(char_count(&right))
            .min(self.rope.len_chars());
        self.rope.remove(at.min(remove_end)..remove_end);
        self.rope.insert(at.min(self.rope.len_chars()), &composed);
        (
            row.saturating_add(rest.len()),
            col.saturating_add(char_count(last)),
        )
    }

    pub fn save(&mut self) -> Result<()> {
        let path = match &self.path {
            Some(p) => p.clone(),
            None => return Err(anyhow::anyhow!("E32: No file name")),
        };
        let mut file = fs::File::create(&path).with_context(|| format!("failed to save {path}"))?;
        self.rope
            .write_to(&mut file)
            .with_context(|| format!("failed to save {path}"))?;
        self.history.mark_clean();
        Ok(())
    }

    pub fn byte_count(&self) -> usize {
        self.rope.len_bytes()
    }

    fn position_to_char(&self, row: usize, col: usize) -> usize {
        if self.rope.len_chars() == 0 {
            return 0;
        }
        let row = row.min(self.raw_line_count().saturating_sub(1));
        let start = self.rope.line_to_char(row);
        start.saturating_add(col.min(self.stripped_len(row)))
    }

    fn line_start_char(&self, row: usize) -> usize {
        if self.rope.len_chars() == 0 {
            return 0;
        }
        self.rope
            .line_to_char(row.min(self.raw_line_count().saturating_sub(1)))
    }

    fn line_end_char(&self, row: usize) -> usize {
        if self.rope.len_chars() == 0 {
            return 0;
        }
        let row = row.min(self.raw_line_count().saturating_sub(1));
        self.line_start_char(row)
            .saturating_add(self.stripped_len(row))
    }

    fn stripped_len(&self, row: usize) -> usize {
        if self.rope.len_chars() == 0 {
            return 0;
        }
        if row >= self.raw_line_count() {
            return 0;
        }
        self.line_cow(row).map_or(0, |line| line.chars().count())
    }

    fn raw_line_count(&self) -> usize {
        self.rope.len_lines()
    }

    fn ends_with_newline(&self) -> bool {
        if self.rope.len_chars() == 0 {
            return false;
        }
        self.rope.get_char(self.rope.len_chars().saturating_sub(1)) == Some('\n')
    }

    fn ensure_line_exists(&mut self, row: usize) {
        while self.raw_line_count() <= row {
            let end = self.rope.len_chars();
            self.rope.insert(end, "\n");
        }
    }

    fn delete_at_line_start(&mut self, row: usize) -> (usize, usize) {
        if row == 0 {
            return (0, 0);
        }
        let prev = row.saturating_sub(1);
        let prev_width = self.line_len(prev);
        let idx = self.line_start_char(row);
        self.rope.remove(idx.saturating_sub(1)..idx);
        self.touch();
        (prev, prev_width)
    }

    fn delete_at_line_end(&mut self, row: usize) {
        if row.saturating_add(1) >= self.len() {
            return;
        }
        let end = self.line_end_char(row);
        self.rope
            .remove(end..end.saturating_add(1).min(self.rope.len_chars()));
        self.touch();
    }

    fn line_span(&self, row: usize) -> (usize, usize) {
        let start = self.line_start_char(row);
        if row.saturating_add(1) < self.raw_line_count() {
            return (start, self.line_start_char(row.saturating_add(1)));
        }
        (start, self.rope.len_chars())
    }
}

fn strip_ending(text: &str) -> std::borrow::Cow<'_, str> {
    let stripped = text.strip_suffix('\n').unwrap_or(text);
    let stripped = stripped.strip_suffix('\r').unwrap_or(stripped);
    if stripped.len() == text.len() {
        return std::borrow::Cow::Borrowed(text);
    }
    std::borrow::Cow::Owned(stripped.to_string())
}

fn strip_owned(mut text: String) -> String {
    if text.ends_with('\n') {
        text.pop();
        if text.ends_with('\r') {
            text.pop();
        }
    }
    text
}

fn slice_from_col(line: &str, col: usize) -> String {
    line.chars().skip(col).collect()
}

fn read_file_text(path: &str) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("failed to open {path}"))
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

fn start_col_of(line: &str) -> usize {
    line.chars()
        .enumerate()
        .find(|(_, ch)| !ch.is_whitespace())
        .map_or(char_count(line), |(idx, _)| idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_string_splits_lines() {
        let b = Buffer::from_string("a\nb");
        assert_eq!(b.len(), 2);
        assert_eq!(b.line(0).as_deref(), Some("a"));
    }

    #[test]
    fn empty_has_one_line() {
        let b = Buffer::from_string("");
        assert_eq!(b.len(), 1);
        assert_eq!(b.line(0).as_deref(), Some(""));
    }

    #[test]
    fn trailing_newline_not_counted() {
        let b = Buffer::from_string("ab\ncd\n");
        assert_eq!(b.len(), 2);
        assert_eq!(b.line(1).as_deref(), Some("cd"));
    }

    #[test]
    fn insert_char_places_char() {
        let mut b = Buffer::from_string("ac");
        b.insert_char(0, 1, 'b');
        assert_eq!(b.line(0).as_deref(), Some("abc"));
    }

    #[test]
    fn insert_newline_splits_line() {
        let mut b = Buffer::from_string("ab");
        b.insert_newline(0, 1);
        assert_eq!(b.len(), 2);
        assert_eq!(b.line(0).as_deref(), Some("a"));
        assert_eq!(b.line(1).as_deref(), Some("b"));
    }

    #[test]
    fn delete_backward_removes_char() {
        let mut b = Buffer::from_string("ab");
        let pos = b.delete_backward(0, 2);
        assert_eq!(b.line(0).as_deref(), Some("a"));
        assert_eq!(pos, (0, 1));
    }

    #[test]
    fn delete_backward_joins_lines() {
        let mut b = Buffer::from_string("a\nb");
        let pos = b.delete_backward(1, 0);
        assert_eq!(b.len(), 1);
        assert_eq!(b.line(0).as_deref(), Some("ab"));
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
    fn save_without_path_errors() {
        let mut b = Buffer::from_string("hi");
        assert!(b.save().is_err());
        assert!(!b.is_dirty());
    }

    #[test]
    fn byte_count_matches_file_bytes() {
        let b = Buffer::from_string("aa\nbb");
        assert_eq!(b.byte_count(), 5);
    }

    #[test]
    fn byte_count_includes_trailing_newline() {
        let b = Buffer::from_string("aa\nbb\n");
        assert_eq!(b.byte_count(), 6);
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
        assert_eq!(b.line(0).as_deref(), Some("ab"));
    }

    #[test]
    fn redo_restores_forward() {
        let mut b = Buffer::from_string("ab");
        b.checkpoint(0, 0);
        b.insert_char(0, 2, 'c');
        b.undo(0, 0);
        assert!(b.redo(0, 0).is_some());
        assert_eq!(b.line(0).as_deref(), Some("abc"));
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
        assert_eq!(b.line(0).as_deref(), Some("b"));
        assert!(b.is_dirty());
    }

    #[test]
    fn delete_forward_joins_lines() {
        let mut b = Buffer::from_string("a\nb");
        b.delete_forward(0, 1);
        assert_eq!(b.len(), 1);
        assert_eq!(b.line(0).as_deref(), Some("ab"));
    }

    #[test]
    fn delete_line_removes_row() {
        let mut b = Buffer::from_string("a\nb\nc");
        b.checkpoint(0, 0);
        b.delete_line(1);
        assert_eq!(b.len(), 2);
        assert_eq!(b.line(1).as_deref(), Some("c"));
        assert!(b.is_dirty());
    }

    #[test]
    fn delete_last_line_keeps_one_empty() {
        let mut b = Buffer::from_string("a");
        b.delete_line(0);
        assert_eq!(b.len(), 1);
        assert_eq!(b.line(0).as_deref(), Some(""));
    }

    #[test]
    fn delete_forward_removes_char_unicode() {
        let mut b = Buffer::from_string("aé");
        b.delete_forward(0, 0);
        assert_eq!(b.line(0).as_deref(), Some("é"));
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
        assert_eq!(b.line(0).as_deref(), Some("aef"));
    }

    #[test]
    fn delete_range_joins_remainders() {
        let mut b = Buffer::from_string("ab\ncd\nef");
        b.delete_range(0, 1, 2, 1);
        assert_eq!(b.len(), 1);
        assert_eq!(b.line(0).as_deref(), Some("a"));
    }

    #[test]
    fn paste_after_inserts_single_line() {
        let mut b = Buffer::from_string("aef");
        assert_eq!(b.paste_after(0, 1, "bcd"), (0, 4));
        assert_eq!(b.line(0).as_deref(), Some("aebcdf"));
    }

    #[test]
    fn paste_before_inserts_at_position() {
        let mut b = Buffer::from_string("def");
        assert_eq!(b.paste_before(0, 0, "abc"), (0, 3));
        assert_eq!(b.line(0).as_deref(), Some("abcdef"));
    }

    #[test]
    fn paste_before_splits_multiple_lines() {
        let mut b = Buffer::from_string("12");
        assert_eq!(b.paste_before(0, 1, "ab\nc"), (1, 2));
        assert_eq!(b.line(0).as_deref(), Some("1ab"));
        assert_eq!(b.line(1).as_deref(), Some("c2"));
    }

    #[test]
    fn paste_after_splits_multiple_lines() {
        let mut b = Buffer::from_string("12");
        assert_eq!(b.paste_after(0, 0, "ab\nc"), (1, 1));
        assert_eq!(b.len(), 2);
        assert_eq!(b.line(0).as_deref(), Some("1ab"));
        assert_eq!(b.line(1).as_deref(), Some("c2"));
    }

    #[test]
    fn paste_empty_changes_nothing() {
        let mut b = Buffer::from_string("ab");
        assert_eq!(b.paste_after(0, 0, ""), (0, 0));
        assert_eq!(b.line(0).as_deref(), Some("ab"));
    }

    #[test]
    fn span_text_excludes_end() {
        let b = Buffer::from_string("foo bar");
        assert_eq!(b.span_text(0, 0, 0, 4), "foo ");
    }

    #[test]
    fn span_text_joins_newlines() {
        let b = Buffer::from_string("ab\ncd");
        assert_eq!(b.span_text(0, 1, 1, 0), "b\n");
    }

    #[test]
    fn delete_span_keeps_end_char() {
        let mut b = Buffer::from_string("foo bar");
        b.delete_span(0, 0, 0, 4);
        assert_eq!(b.line(0).as_deref(), Some("bar"));
    }

    #[test]
    fn delete_span_joins_lines() {
        let mut b = Buffer::from_string("foo\nbar");
        b.delete_span(0, 0, 1, 0);
        assert_eq!(b.len(), 1);
        assert_eq!(b.line(0).as_deref(), Some("bar"));
    }

    #[test]
    fn paste_lines_below_inserts() {
        let mut b = Buffer::from_string("aa\nbb\ncc");
        assert_eq!(b.paste_lines_below(1, &["bb".to_string()]), 2);
        assert_eq!(b.len(), 4);
        assert_eq!(b.line(2).as_deref(), Some("bb"));
    }

    #[test]
    fn clear_line_empties() {
        let mut b = Buffer::from_string("abcdef");
        b.clear_line(0);
        assert_eq!(b.line(0).as_deref(), Some(""));
        assert_eq!(b.len(), 1);
    }

    #[test]
    fn first_non_blank_skips_space() {
        assert_eq!(first_non_blank("  ind"), 2);
        assert_eq!(first_non_blank("ab"), 0);
        assert_eq!(first_non_blank(""), 0);
    }

    #[test]
    fn insert_start_col_prefers_content() {
        let b = Buffer::from_string("  abc\n   \nxyz");
        assert_eq!(b.insert_start_col(0), 2);
        assert_eq!(b.insert_start_col(1), 3);
        assert_eq!(b.insert_start_col(2), 0);
    }

    #[test]
    fn char_index_roundtrips_positions() {
        let b = Buffer::from_string("ab\ncd");
        assert_eq!(b.total_chars(), 5);
        assert_eq!(b.char_index_of(0, 0), 0);
        assert_eq!(b.char_index_of(0, 2), 2);
        assert_eq!(b.char_index_of(1, 1), 4);
        assert_eq!(b.position_of_char(0), (0, 0));
        assert_eq!(b.position_of_char(4), (1, 1));
        assert_eq!(b.position_of_char(5), (1, 2));
        assert_eq!(b.position_of_char(99), (1, 2));
    }

    #[test]
    fn position_of_char_handles_empty() {
        let b = Buffer::from_string("");
        assert_eq!(b.total_chars(), 0);
        assert_eq!(b.position_of_char(0), (0, 0));
    }

    #[test]
    fn position_of_char_handles_trailing_newline() {
        let b = Buffer::from_string("ab\ncd\n");
        assert_eq!(b.position_of_char(b.total_chars()), (2, 0));
        assert_eq!(b.position_of_char(0), (0, 0));
    }

    #[test]
    fn revision_starts_at_zero() {
        let b = Buffer::from_string("ab");
        assert_eq!(b.revision(), 0);
    }

    #[test]
    fn revision_bumps_on_edit() {
        let mut b = Buffer::from_string("ab");
        b.insert_char(0, 2, 'c');
        assert_eq!(b.revision(), 1);
        b.delete_backward(0, 3);
        assert_eq!(b.revision(), 2);
    }

    #[test]
    fn revision_holds_without_edit() {
        let mut b = Buffer::from_string("ab");
        let _ = b.line(0);
        let _ = b.position_of_char(1);
        assert_eq!(b.revision(), 0);
        b.checkpoint(0, 0);
        assert_eq!(b.revision(), 0);
    }

    #[test]
    fn revision_bumps_on_undo_restoring() {
        let mut b = Buffer::from_string("ab");
        b.checkpoint(0, 0);
        b.insert_char(0, 2, 'c');
        let before = b.revision();
        assert!(b.undo(0, 0).is_some());
        assert_eq!(b.revision(), before.saturating_add(1));
        assert!(b.redo(0, 0).is_some());
        assert_eq!(b.revision(), before.saturating_add(2));
    }
}
