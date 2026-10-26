use super::indent::engine::{closer_width, next_width, IndentRequest, INDENT_WINDOW_LINES};
use super::indent::reindent_line;
use super::Editor;

impl Editor {
    pub fn set_autoindent(&mut self, autoindent: bool) {
        self.autoindent = autoindent;
    }

    pub fn autoindent(&self) -> bool {
        self.autoindent
    }

    pub(super) fn enter_autoindent(&mut self, row: usize, width: usize) {
        if !self.autoindent {
            return;
        }
        let indent = self.indent_text(width);
        let tail = self
            .buffer
            .line(row)
            .map(|line| line.trim_start().to_string())
            .unwrap_or_default();
        self.set_line_text(row, &format!("{indent}{tail}"));
        self.cursor.set_position(row, indent.chars().count());
        self.pending_autoindent = Some((row, indent));
    }

    pub(super) fn clear_autoindent_line(&mut self) {
        let Some((row, indent)) = self.pending_autoindent.take() else {
            return;
        };
        if self.buffer.line(row).as_deref() != Some(indent.as_str()) {
            return;
        }
        self.set_line_text(row, "");
        if self.cursor.row() == row {
            self.cursor.set_position(row, 0);
        }
    }

    fn set_line_text(&mut self, row: usize, text: &str) {
        let width = self.buffer.line_len(row);
        if width > 0 {
            self.buffer
                .delete_range(row, 0, row, width.saturating_sub(1));
        }
        for (index, ch) in text.chars().enumerate() {
            self.buffer.insert_char(row, index, ch);
        }
    }

    pub(super) fn indent_after_line(&self, row: usize) -> Option<String> {
        if !self.autoindent {
            return None;
        }
        let width = self.smart_indent_width(row, self.line_width(row), self.line_indent_width(row));
        Some(self.indent_text(width))
    }

    pub(super) fn indent_before_line(&self, row: usize) -> Option<String> {
        if !self.autoindent {
            return None;
        }
        let width = self.smart_indent_width(row, 0, self.line_indent_width(row));
        Some(self.indent_text(width))
    }

    pub(super) fn newline_indent_width(&self, row: usize, col: usize) -> usize {
        let current_width = self.prefix_indent_width(row, col);
        self.smart_indent_width(row, col, current_width)
    }

    pub(super) fn smart_indent_width(&self, row: usize, col: usize, current_width: usize) -> usize {
        if !self.autoindent {
            return current_width;
        }
        let (start, lines) = self.indent_window(row);
        let request = self.indent_request(&lines, row - start, col, current_width);
        next_width(&request)
    }

    pub(super) fn dedent_before_closer(&mut self, closer: char) {
        if !self.autoindent || self.mode != crate::mode::Mode::Insert || !is_closing_bracket(closer)
        {
            return;
        }
        let (row, col) = (self.cursor.row(), self.cursor.col());
        if !self.line_prefix_is_blank(row, col) {
            return;
        }
        let Some(width) = self.closer_indent_width(row, col, closer) else {
            return;
        };
        self.set_line_indent(row, width);
    }

    fn closer_indent_width(&self, row: usize, col: usize, closer: char) -> Option<usize> {
        let (start, lines) = self.indent_window(row);
        let request = self.indent_request(&lines, row - start, col, 0);
        closer_width(&request, closer)
    }

    fn indent_window(&self, row: usize) -> (usize, Vec<String>) {
        let start = row.saturating_sub(INDENT_WINDOW_LINES - 1);
        let lines = (start..=row)
            .map(|row| self.buffer.line(row).unwrap_or_default())
            .collect();
        (start, lines)
    }

    fn indent_request<'a>(
        &'a self,
        lines: &'a [String],
        row: usize,
        col: usize,
        current_width: usize,
    ) -> IndentRequest<'a> {
        let indent = self.effective_indent();
        IndentRequest {
            lines,
            row,
            col,
            current_width,
            shiftwidth: indent.shift_unit(),
            tabstop: indent.tabstop,
            rules: &self.language_indent.rules,
            editing: &self.language_editing,
            language: self.language_indent.language,
        }
    }

    fn line_indent_width(&self, row: usize) -> usize {
        self.buffer
            .indent_width(row, self.effective_indent().tabstop)
    }

    fn line_prefix_is_blank(&self, row: usize, col: usize) -> bool {
        self.buffer
            .line(row)
            .is_some_and(|line| line.chars().take(col).all(|ch| ch.is_whitespace()))
    }

    fn set_line_indent(&mut self, row: usize, width: usize) {
        if self.line_indent_width(row) == width {
            return;
        }
        let indent = self.effective_indent();
        let line = self.buffer.line(row).unwrap_or_default();
        let next = reindent_line(&line, width, indent.expandtab, indent.tabstop);
        self.record_maybe();
        self.buffer.replace_lines(row, row, &next);
        self.cursor
            .set_position(row, self.indent_text(width).chars().count());
    }

    pub(super) fn prefix_indent_width(&self, row: usize, col: usize) -> usize {
        let tabstop = self.effective_indent().tabstop;
        let end = self.buffer.indent_end(row).min(col);
        self.buffer.screen_col(row, end, tabstop)
    }

    pub(super) fn backspace_indent(&mut self) -> bool {
        if self.mode != crate::mode::Mode::Insert || !self.autoindent {
            return false;
        }
        let (row, col) = (self.cursor.row(), self.cursor.col());
        if col == 0 {
            return false;
        }
        let Some(line) = self.buffer.line(row) else {
            return false;
        };
        let chars: Vec<char> = line.chars().collect();
        if col > chars.len() || !chars[..col].iter().all(|ch| ch.is_whitespace()) {
            return false;
        }
        let indent = self.effective_indent();
        let step = indent.shift_unit().max(1);
        let width = self.buffer.screen_col(row, col, indent.tabstop);
        if width == 0 {
            return false;
        }
        let want = (width.saturating_sub(1) / step) * step;
        let target = indent_target(&chars[..col], want, indent.tabstop);
        if target >= col {
            return false;
        }
        self.buffer.delete_range(row, target, row, col - 1);
        self.cursor.set_position(row, target);
        true
    }

    pub(super) fn indent_text(&self, width: usize) -> String {
        let indent = self.effective_indent();
        if indent.expandtab {
            return " ".repeat(width);
        }
        let tabstop = indent.tabstop.max(1);
        let mut text = String::new();
        let mut col = 0;
        while col + tabstop <= width {
            text.push('\t');
            col += tabstop;
        }
        text.push_str(&" ".repeat(width.saturating_sub(col)));
        text
    }
}

fn is_closing_bracket(ch: char) -> bool {
    matches!(ch, ')' | ']' | '}')
}

fn indent_target(chars: &[char], want: usize, tabstop: usize) -> usize {
    let mut vcol = 0;
    for (index, ch) in chars.iter().enumerate() {
        let size = if *ch == '\t' {
            tabstop.max(1) - vcol % tabstop.max(1)
        } else {
            1
        };
        if vcol + size > want {
            return index;
        }
        vcol += size;
    }
    chars.len()
}

#[cfg(test)]
mod tests;
