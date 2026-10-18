use anyhow::{Context, Result};
use ropey::Rope;
use std::fs;
use std::io::Write;

use crate::history::History;
use crate::line_adjust::{self, LineAdjust};

pub struct Buffer {
    rope: Rope,
    pub path: Option<String>,
    history: History,
    revision: u64,
    line_changes: u64,
    adjustments: Vec<LineAdjust>,
}

impl Buffer {
    pub fn from_string(s: &str) -> Self {
        Self {
            rope: Rope::from_str(s),
            path: None,
            history: History::new(),
            revision: 0,
            line_changes: 0,
            adjustments: Vec::new(),
        }
    }

    pub fn from_file(path: &str) -> Result<Self> {
        let text = read_file_text(path)?;
        Ok(Self::from_text_with_path(&text, path))
    }

    pub fn open(path: &str) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(text) => Ok(Self::from_text_with_path(&text, path)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let mut buffer = Self::from_string("");
                buffer.path = Some(path.to_string());
                Ok(buffer)
            }
            Err(error) => Err(error).with_context(|| format!("failed to open {path}")),
        }
    }

    fn from_text_with_path(text: &str, path: &str) -> Self {
        let mut buffer = Self::from_string(text);
        buffer.path = Some(path.to_string());
        buffer
    }

    pub fn reload_from_disk(&mut self) -> Result<()> {
        let Some(path) = self.path.clone() else {
            anyhow::bail!("no file name");
        };
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => {
                return Err(error).with_context(|| format!("failed to open {path}"));
            }
        };
        self.rope = Rope::from_str(&text);
        self.touch();
        Ok(())
    }

    pub fn mark_clean(&mut self) {
        self.history.mark_clean();
    }

    pub fn has_path(&self) -> bool {
        self.path.is_some()
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    fn touch(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn len(&self) -> usize {
        line_adjust::logical_line_count(&self.rope)
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
        self.history
            .checkpoint(&self.rope, row, col, self.line_changes);
    }

    pub fn maybe_checkpoint(&mut self, row: usize, col: usize) {
        self.history
            .maybe_checkpoint(&self.rope, row, col, self.line_changes);
    }

    pub fn undo(&mut self, row: usize, col: usize) -> Option<(usize, usize)> {
        let (restored, adjust) = self
            .history
            .undo(&mut self.rope, row, col, self.line_changes)?;
        self.push_adjustment(adjust);
        self.touch();
        Some(restored)
    }

    pub fn redo(&mut self, row: usize, col: usize) -> Option<(usize, usize)> {
        let (restored, adjust) = self
            .history
            .redo(&mut self.rope, row, col, self.line_changes)?;
        self.push_adjustment(adjust);
        self.touch();
        Some(restored)
    }

    pub fn redo_keep_cursor(&mut self, row: usize, col: usize) -> bool {
        let Some((_, adjust)) = self
            .history
            .redo(&mut self.rope, row, col, self.line_changes)
        else {
            return false;
        };
        self.push_adjustment(adjust);
        self.touch();
        true
    }

    pub fn take_line_adjustments(&mut self) -> Vec<LineAdjust> {
        std::mem::take(&mut self.adjustments)
    }

    fn push_adjustment(&mut self, adjust: Option<LineAdjust>) {
        if let Some(adjust) = adjust {
            self.adjustments.push(adjust);
            self.line_changes = self.line_changes.wrapping_add(1);
        }
    }

    pub fn insert_char(&mut self, row: usize, col: usize, ch: char) {
        self.ensure_line_exists(row);
        let idx = self.position_to_char(row, col);
        self.insert_text(idx, &ch.to_string());
        self.touch();
    }

    pub fn insert_newline(&mut self, row: usize, col: usize) {
        self.ensure_line_exists(row);
        let idx = self.position_to_char(row, col);
        self.insert_text(idx, "\n");
        self.touch();
    }

    pub fn delete_backward(&mut self, row: usize, col: usize) -> (usize, usize) {
        if row == self.len() && col == 0 && self.ends_with_newline() {
            let last = self.rope.len_chars();
            self.remove_text(last.saturating_sub(1), last);
            self.touch();
            let prev = row.saturating_sub(1);
            return (prev, self.line_len(prev));
        }
        if row >= self.len() {
            return (row, col);
        }
        if col > 0 {
            let idx = self.position_to_char(row, col);
            self.remove_text(idx.saturating_sub(1), idx);
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
            self.remove_text(idx, idx.saturating_add(1).min(self.rope.len_chars()));
            self.touch();
            return;
        }
        self.delete_at_line_end(row);
    }

    pub fn delete_line(&mut self, row: usize) {
        if self.len() <= 1 {
            self.remove_row_range(0, 0);
            return;
        }
        self.remove_row_range(row, row);
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
            self.remove_text(start, end);
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
        if self.rope.len_chars() == 0 {
            let composed = lines.join("\n");
            self.insert_text(0, &composed);
            self.touch();
            return 0;
        }
        let row = row.min(self.len().saturating_sub(1));
        let at = self.line_end_char(row);
        let mut composed = String::new();
        for line in lines {
            composed.push('\n');
            composed.push_str(line);
        }
        self.insert_text(at.min(self.rope.len_chars()), &composed);
        self.touch();
        row.saturating_add(1)
    }

    pub fn paste_lines_above(&mut self, row: usize, lines: &[String]) -> usize {
        let row = row.min(self.len().saturating_sub(1));
        let at = self.line_start_char(row);
        let mut composed = String::new();
        for line in lines {
            composed.push_str(line);
            composed.push('\n');
        }
        self.insert_text(at.min(self.rope.len_chars()), &composed);
        self.touch();
        row
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

    pub fn indent_end(&self, row: usize) -> usize {
        match self.line_cow(row) {
            None => 0,
            Some(line) => line
                .chars()
                .take_while(|ch| *ch == ' ' || *ch == '\t')
                .count(),
        }
    }

    pub fn indent_width(&self, row: usize, tabstop: usize) -> usize {
        match self.line_cow(row) {
            None => 0,
            Some(line) => indent_width_of(&line, tabstop),
        }
    }

    pub fn screen_col(&self, row: usize, col: usize, tabstop: usize) -> usize {
        let mut screen = 0;
        if let Some(line) = self.line_cow(row) {
            for ch in line.chars().take(col) {
                screen += tab_width(ch, screen, tabstop);
            }
        }
        screen
    }

    pub fn char_at(&self, row: usize, col: usize) -> Option<char> {
        self.line_cow(row)?.chars().nth(col)
    }

    pub fn char_col(&self, row: usize, screen: usize, tabstop: usize) -> usize {
        let width = self.line_len(row);
        if width == 0 {
            return 0;
        }
        let mut screen_pos = 0;
        let mut found = 0;
        if let Some(line) = self.line_cow(row) {
            for (index, ch) in line.chars().enumerate() {
                if screen_pos > screen {
                    break;
                }
                found = index;
                screen_pos += tab_width(ch, screen_pos, tabstop);
            }
        }
        found.min(width.saturating_sub(1))
    }

    pub fn shift_case_range(
        &mut self,
        start_row: usize,
        start_col: usize,
        end_row: usize,
        end_col: usize,
        shift: fn(char) -> char,
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
        if start >= end {
            return;
        }
        let mapped: String = self.rope.slice(start..end).chars().map(shift).collect();
        self.rewrite_text(start, end, &mapped);
        self.touch();
    }

    pub fn lines_text(&self, first: usize, last: usize) -> String {
        if first >= self.len() {
            return String::new();
        }
        let last = last.min(self.len().saturating_sub(1));
        if first > last {
            return String::new();
        }
        let start = self.line_start_char(first);
        let end = self.position_to_char(last, self.line_len(last));
        self.rope.slice(start..end.max(start)).to_string()
    }

    pub fn delete_lines(&mut self, first: usize, last: usize) {
        self.remove_row_range(first, last);
    }

    pub fn replace_lines(&mut self, first: usize, last: usize, text: &str) {
        if first >= self.len() {
            return;
        }
        let last = last.min(self.len().saturating_sub(1));
        if first > last {
            return;
        }
        let start = self.line_start_char(first);
        let end = self.position_to_char(last, self.line_len(last));
        self.rewrite_text(start, end, text);
        self.touch();
    }

    fn line_span_range(&self, first: usize, last: usize) -> Option<(usize, usize)> {
        if first >= self.len() {
            return None;
        }
        let last = last.min(self.len().saturating_sub(1));
        if first > last {
            return None;
        }
        let start = self.line_start_char(first);
        let end = if last.saturating_add(1) < self.raw_line_count() {
            self.line_start_char(last.saturating_add(1))
        } else {
            self.rope.len_chars()
        };
        Some((start, end))
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
            self.remove_text(start, end);
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
            self.remove_text(start, end);
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
        self.insert_text(at.min(self.rope.len_chars()), fragment);
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
        self.insert_text(at.min(self.rope.len_chars()), fragment);
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
        self.remove_text(at.min(remove_end), remove_end);
        self.insert_text(at.min(self.rope.len_chars()), &composed);
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
        self.remove_text(at.min(remove_end), remove_end);
        self.insert_text(at.min(self.rope.len_chars()), &composed);
        (
            row.saturating_add(rest.len()),
            col.saturating_add(char_count(last)),
        )
    }

    pub fn save(&mut self) -> Result<()> {
        let Some(path) = self.path.clone() else {
            anyhow::bail!("E32: No file name");
        };
        self.write_to(&path)?;
        self.history.mark_clean();
        Ok(())
    }

    pub fn write_to(&self, path: &str) -> Result<()> {
        let mut file = fs::File::create(path).with_context(|| format!("failed to save {path}"))?;
        self.rope
            .write_to(&mut file)
            .with_context(|| format!("failed to save {path}"))?;
        if self.needs_final_newline() {
            file.write_all(b"\n")
                .with_context(|| format!("failed to save {path}"))?;
        }
        Ok(())
    }

    pub fn file_bytes(&self) -> usize {
        self.rope.len_bytes() + usize::from(self.needs_final_newline())
    }

    fn needs_final_newline(&self) -> bool {
        let bytes = self.rope.len_bytes();
        bytes > 0 && self.rope.chars().last() != Some('\n')
    }

    pub fn adopt_path(&mut self, path: &str) {
        self.path = Some(path.to_string());
        self.history.mark_clean();
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
            self.insert_text(end, "\n");
        }
    }

    fn insert_text(&mut self, index: usize, text: &str) {
        self.push_adjustment(line_adjust::insertion(&self.rope, index, text));
        self.rope.insert(index, text);
    }

    fn remove_text(&mut self, start: usize, end: usize) {
        self.push_adjustment(line_adjust::char_removal(&self.rope, start, end));
        self.rope.remove(start..end);
    }

    fn remove_row_range(&mut self, first: usize, last: usize) {
        if first >= self.len() {
            return;
        }
        let last = last.min(self.len().saturating_sub(1));
        if first > last {
            return;
        }
        let Some((start, end)) = self.line_span_range(first, last) else {
            return;
        };
        if start < end {
            let start = if end == self.rope.len_chars()
                && first > 0
                && start > 0
                && self.line_len(first - 1) > 0
                && self.rope.get_char(start - 1) == Some('\n')
            {
                start - 1
            } else {
                start
            };
            self.push_adjustment(Some(line_adjust::row_removal(first, last)));
            self.rope.remove(start..end);
            self.touch();
        }
    }

    fn rewrite_text(&mut self, start: usize, end: usize, text: &str) {
        self.rope.remove(start..end.max(start));
        self.rope.insert(start.min(self.rope.len_chars()), text);
    }

    fn delete_at_line_start(&mut self, row: usize) -> (usize, usize) {
        if row == 0 {
            return (0, 0);
        }
        let prev = row.saturating_sub(1);
        let prev_width = self.line_len(prev);
        let idx = self.line_start_char(row);
        self.remove_text(idx.saturating_sub(1), idx);
        self.touch();
        (prev, prev_width)
    }

    fn delete_at_line_end(&mut self, row: usize) {
        if row.saturating_add(1) >= self.len() {
            return;
        }
        let end = self.line_end_char(row);
        self.remove_text(end, end.saturating_add(1).min(self.rope.len_chars()));
        self.touch();
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

fn tab_width(ch: char, screen: usize, tabstop: usize) -> usize {
    if ch != '\t' {
        return 1;
    }
    let tabstop = tabstop.max(1);
    tabstop - screen % tabstop
}

fn read_file_text(path: &str) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("failed to open {path}"))
}

fn char_count(text: &str) -> usize {
    text.chars().count()
}

pub fn indent_width_of(line: &str, tabstop: usize) -> usize {
    let mut width = 0;
    for ch in line.chars() {
        if ch != ' ' && ch != '\t' {
            break;
        }
        width += tab_width(ch, width, tabstop);
    }
    width
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
mod adjust_tests {
    use super::*;

    fn adjust(
        line1: usize,
        line2: usize,
        amount: Option<isize>,
        amount_after: isize,
    ) -> LineAdjust {
        LineAdjust {
            line1,
            line2,
            amount,
            amount_after,
        }
    }

    #[test]
    fn delete_line_records_row_adjustment() {
        let mut buffer = Buffer::from_string("a\nb\nc");
        buffer.delete_line(1);
        assert_eq!(buffer.take_line_adjustments(), vec![adjust(2, 2, None, -1)]);
        assert!(buffer.take_line_adjustments().is_empty());
    }

    #[test]
    fn delete_lines_records_row_adjustment() {
        let mut buffer = Buffer::from_string("a\nb\nc\nd");
        buffer.delete_lines(1, 2);
        assert_eq!(buffer.take_line_adjustments(), vec![adjust(2, 3, None, -2)]);
    }

    #[test]
    fn insert_newline_records_insertion_adjustment() {
        let mut buffer = Buffer::from_string("ab\ncd");
        buffer.insert_newline(0, 1);
        assert_eq!(
            buffer.take_line_adjustments(),
            vec![adjust(2, usize::MAX, Some(1), 0)]
        );
    }

    #[test]
    fn paste_lines_below_records_insertion_adjustment() {
        let mut buffer = Buffer::from_string("a\nb");
        buffer.paste_lines_below(0, &["x".to_string(), "y".to_string()]);
        assert_eq!(
            buffer.take_line_adjustments(),
            vec![adjust(2, usize::MAX, Some(2), 0)]
        );
    }

    #[test]
    fn paste_lines_above_records_insertion_adjustment() {
        let mut buffer = Buffer::from_string("a\nb");
        buffer.paste_lines_above(1, &["x".to_string()]);
        assert_eq!(
            buffer.take_line_adjustments(),
            vec![adjust(2, usize::MAX, Some(1), 0)]
        );
    }

    #[test]
    fn delete_span_across_lines_records_char_adjustment() {
        let mut buffer = Buffer::from_string("ab\ncd\nef");
        buffer.delete_span(0, 1, 1, 1);
        assert_eq!(buffer.take_line_adjustments(), vec![adjust(2, 2, None, -1)]);
    }

    #[test]
    fn char_edits_record_nothing() {
        let mut buffer = Buffer::from_string("ab\ncd");
        buffer.insert_char(0, 1, 'x');
        buffer.delete_range(0, 0, 0, 0);
        buffer.clear_line(1);
        assert!(buffer.take_line_adjustments().is_empty());
    }

    #[test]
    fn replace_lines_records_nothing() {
        let mut buffer = Buffer::from_string("  a\n  b\n  c");
        buffer.replace_lines(0, 2, "    a\n    b\n    c");
        assert!(buffer.take_line_adjustments().is_empty());
    }

    #[test]
    fn undo_without_line_changes_records_nothing() {
        let mut buffer = Buffer::from_string("a\nb");
        buffer.checkpoint(0, 0);
        buffer.delete_range(1, 0, 1, 0);
        buffer.take_line_adjustments();
        buffer.undo(0, 0);
        assert!(buffer.take_line_adjustments().is_empty());
    }

    #[test]
    fn undo_records_adjustment() {
        let mut buffer = Buffer::from_string("a\nb\nc");
        buffer.checkpoint(0, 0);
        buffer.delete_line(1);
        buffer.take_line_adjustments();
        buffer.undo(0, 0);
        assert_eq!(buffer.take_line_adjustments(), vec![adjust(2, 1, None, 1)]);
    }
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
    fn indent_width_counts_tabs() {
        let b = Buffer::from_string("  x\n\ty\n  \tz");
        assert_eq!(b.indent_width(0, 8), 2);
        assert_eq!(b.indent_width(1, 8), 8);
        assert_eq!(b.indent_width(2, 8), 8);
    }

    #[test]
    fn indent_end_counts_chars() {
        let b = Buffer::from_string("  \tx");
        assert_eq!(b.indent_end(0), 3);
    }

    #[test]
    fn screen_col_expands_tabs() {
        let b = Buffer::from_string("\tfoo");
        assert_eq!(b.screen_col(0, 0, 8), 0);
        assert_eq!(b.screen_col(0, 1, 8), 8);
        assert_eq!(b.screen_col(0, 2, 8), 9);
    }

    #[test]
    fn char_at_returns_char_by_column() {
        let b = Buffer::from_string("a\tb\ncd");
        assert_eq!(b.char_at(0, 0), Some('a'));
        assert_eq!(b.char_at(0, 1), Some('\t'));
        assert_eq!(b.char_at(0, 2), Some('b'));
        assert_eq!(b.char_at(0, 3), None);
        assert_eq!(b.char_at(1, 1), Some('d'));
        assert_eq!(b.char_at(2, 0), None);
    }

    #[test]
    fn char_col_snaps_tab_interior() {
        let b = Buffer::from_string("\tfoo");
        assert_eq!(b.char_col(0, 0, 8), 0);
        assert_eq!(b.char_col(0, 1, 8), 0);
        assert_eq!(b.char_col(0, 8, 8), 1);
        assert_eq!(b.char_col(0, 99, 8), 3);
    }

    #[test]
    fn paste_before_splits_multiple_lines() {
        let mut b = Buffer::from_string("12");
        assert_eq!(b.paste_before(0, 1, "ab\nc"), (1, 2));
        assert_eq!(b.line(0).as_deref(), Some("1ab"));
        assert_eq!(b.line(1).as_deref(), Some("c2"));
    }

    #[test]
    fn shift_case_range_toggles_single_line() {
        let mut b = Buffer::from_string("aBcDeF");
        b.shift_case_range(0, 1, 0, 3, |ch| {
            if ch.is_ascii_lowercase() {
                ch.to_ascii_uppercase()
            } else {
                ch.to_ascii_lowercase()
            }
        });
        assert_eq!(b.line(0).as_deref(), Some("abCdeF"));
    }

    #[test]
    fn shift_case_range_spans_lines() {
        let mut b = Buffer::from_string("aBcDeF\ngHiJkL");
        b.shift_case_range(0, 1, 1, 1, |ch| ch.to_ascii_lowercase());
        assert_eq!(b.line(0).as_deref(), Some("abcdef"));
        assert_eq!(b.line(1).as_deref(), Some("ghiJkL"));
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
    fn paste_lines_above_inserts() {
        let mut b = Buffer::from_string("aa\nbb\ncc");
        assert_eq!(b.paste_lines_above(1, &["bb".to_string()]), 1);
        assert_eq!(b.len(), 4);
        assert_eq!(b.line(1).as_deref(), Some("bb"));
        assert_eq!(b.line(2).as_deref(), Some("bb"));
    }

    #[test]
    fn paste_lines_above_empty_buffer_inserts() {
        let mut b = Buffer::from_string("");
        assert_eq!(b.paste_lines_above(0, &[String::new()]), 0);
        assert_eq!(b.window_lines(0, 3), vec![String::new(), String::new()]);
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
