use super::lexical::{LexicalContext, LexicalScanner};
use super::session::InsertEdit;
use super::Editor;
use crate::editing::Pair;
use crate::mode::Mode;
use crate::words::is_word_char;

impl Editor {
    pub(super) fn try_auto_pair(&mut self, ch: char) -> bool {
        if !self.auto_pairs || self.mode != Mode::Insert {
            return false;
        }
        if self.overtype_closer(ch) {
            return true;
        }
        let Some(pair) = self.opening_pair(ch) else {
            return false;
        };
        if !self.can_insert_pair(pair) {
            return false;
        }
        self.insert_pair(pair);
        true
    }

    pub(super) fn try_pair_newline(&mut self) -> bool {
        if !self.auto_pairs || self.mode != Mode::Insert {
            return false;
        }
        let (row, col) = (self.cursor.row(), self.cursor.col());
        let Some(open) = self.previous_char(row, col) else {
            return false;
        };
        let Some(close) = self.buffer.char_at(row, col) else {
            return false;
        };
        let Some(pair) = self.opening_pair(open) else {
            return false;
        };
        if pair.is_quote() || pair.close != close {
            return false;
        }
        let base = self.line_indent(row);
        let current_width = self
            .buffer
            .screen_col(row, base.chars().count(), self.tabstop());
        let width = self.smart_indent_width(row, col, current_width);
        let inner = self.indent_text(width);
        self.record_boundary_at(row, col);
        self.split_pair_line(&base, &inner);
        self.record_insert(InsertEdit::PairNewline { base, inner });
        true
    }

    pub(super) fn try_pair_backspace(&mut self) -> bool {
        if !self.auto_pairs || self.mode != Mode::Insert {
            return false;
        }
        let (row, col) = (self.cursor.row(), self.cursor.col());
        if col == 0 {
            return false;
        }
        let Some(open) = self.buffer.char_at(row, col - 1) else {
            return false;
        };
        let Some(close) = self.buffer.char_at(row, col) else {
            return false;
        };
        let Some(pair) = self.opening_pair(open) else {
            return false;
        };
        if pair.close != close {
            return false;
        }
        self.record_maybe();
        self.record_insert(InsertEdit::PairBackspace { open, close });
        self.buffer.delete_range(row, col - 1, row, col);
        self.cursor.set_position(row, col - 1);
        true
    }

    pub(super) fn surround_selection(&mut self, open: char) {
        if !self.mode.is_visual() || self.mode == Mode::VisualBlock {
            return;
        }
        let Some(pair) = self.opening_pair(open) else {
            return;
        };
        let Some((start, end)) = self.selection_range() else {
            return;
        };
        self.note_visual_repeat(crate::actions::Action::SurroundSelection { open });
        self.surround_range(start, end, pair, self.mode == Mode::VisualLine);
        self.exit_visual();
    }

    pub(super) fn replay_insert_pair(&mut self, pair: Pair) {
        self.record_maybe();
        let (row, col) = (self.cursor.row(), self.cursor.col());
        self.buffer.insert_char(row, col, pair.open);
        self.buffer.insert_char(row, col + 1, pair.close);
        self.cursor.set_position(row, col + 1);
    }

    pub(super) fn replay_overtype(&mut self, ch: char) {
        if self.buffer.char_at(self.cursor.row(), self.cursor.col()) == Some(ch) {
            self.cursor.advance_col();
            return;
        }
        self.insert_literal_char(ch);
    }

    pub(super) fn replay_pair_backspace(&mut self, open: char, close: char) {
        let (row, col) = (self.cursor.row(), self.cursor.col());
        if col > 0
            && self.buffer.char_at(row, col - 1) == Some(open)
            && self.buffer.char_at(row, col) == Some(close)
        {
            self.record_maybe();
            self.buffer.delete_range(row, col - 1, row, col);
            self.cursor.set_position(row, col - 1);
            return;
        }
        self.delete_backward_literal();
    }

    pub(super) fn replay_pair_newline(&mut self, base: &str, inner: &str) {
        self.split_pair_line(base, inner);
        self.record_insert(InsertEdit::PairNewline {
            base: base.to_string(),
            inner: inner.to_string(),
        });
    }

    pub(super) fn opening_pair(&self, open: char) -> Option<Pair> {
        self.language_editing
            .pairs
            .iter()
            .copied()
            .find(|pair| pair.open == open)
    }

    fn is_closer(&self, ch: char) -> bool {
        self.language_editing
            .pairs
            .iter()
            .any(|pair| pair.contains_closer(ch))
    }

    fn overtype_closer(&mut self, ch: char) -> bool {
        if !self.is_closer(ch) {
            return false;
        }
        if self.buffer.char_at(self.cursor.row(), self.cursor.col()) != Some(ch) {
            return false;
        }
        self.record_insert(InsertEdit::Overtype(ch));
        self.cursor.advance_col();
        true
    }

    fn can_insert_pair(&self, pair: Pair) -> bool {
        let (row, col) = (self.cursor.row(), self.cursor.col());
        if !self.safe_closer_position(row, col) {
            return false;
        }
        if !pair.is_quote() {
            return true;
        }
        if self.previous_char(row, col).is_some_and(is_word_char) {
            return false;
        }
        !self.inside_string_or_comment(row, col)
    }

    fn safe_closer_position(&self, row: usize, col: usize) -> bool {
        match self.buffer.char_at(row, col) {
            None => true,
            Some(ch) => ch.is_whitespace() || matches!(ch, ')' | ']' | '}' | '>'),
        }
    }

    fn insert_pair(&mut self, pair: Pair) {
        let (row, col) = (self.cursor.row(), self.cursor.col());
        self.record_boundary_at(row, col);
        self.record_insert(InsertEdit::Pair {
            open: pair.open,
            close: pair.close,
        });
        self.buffer.insert_char(row, col, pair.open);
        self.buffer.insert_char(row, col + 1, pair.close);
        self.cursor.set_position(row, col + 1);
        self.buffer.begin_change();
    }

    fn split_pair_line(&mut self, base: &str, inner: &str) {
        let row = self.cursor.row();
        let col = self.cursor.col();
        let line = self.buffer.line(row).unwrap_or_default();
        let chars: Vec<char> = line.chars().collect();
        let col = col.min(chars.len());
        let prefix: String = chars[..col].iter().collect();
        let tail: String = chars[col..].iter().collect();
        let text = format!("{prefix}\n{inner}\n{base}{tail}");
        self.record_maybe();
        self.buffer.replace_lines(row, row, &text);
        self.cursor
            .set_position(row.saturating_add(1), inner.chars().count());
        self.clear_autoindent_line();
    }

    fn line_indent(&self, row: usize) -> String {
        let line = self.buffer.line(row).unwrap_or_default();
        line.chars().take_while(|ch| ch.is_whitespace()).collect()
    }

    fn previous_char(&self, row: usize, col: usize) -> Option<char> {
        col.checked_sub(1)
            .and_then(|previous| self.buffer.char_at(row, previous))
    }

    fn inside_string_or_comment(&self, row: usize, col: usize) -> bool {
        // TODO: replace the bounded lexical scan with tree-sitter context when available.
        let start_row = self.context_scan_start(row);
        let context =
            LexicalContext::with_language(&self.language_editing, self.language_indent.language);
        let mut scanner = LexicalScanner::new(context);
        for current_row in start_row..=row {
            let line = self.buffer.line(current_row).unwrap_or_default();
            let chars: Vec<char> = line.chars().collect();
            let end = if current_row == row {
                col.min(chars.len())
            } else {
                chars.len()
            };
            for index in 0..end {
                scanner.advance(&chars, index);
            }
            if current_row < row {
                scanner.finish_line();
            }
        }
        !scanner.in_code()
    }

    fn context_scan_start(&self, row: usize) -> usize {
        let mut budget = 64 * 1024;
        let mut start = row;
        while start > 0 && row - start < 512 {
            let width = self.buffer.line_len(start - 1).saturating_add(1);
            if width > budget {
                break;
            }
            budget -= width;
            start -= 1;
        }
        start
    }

    pub(super) fn surround_range(
        &mut self,
        start: (usize, usize),
        end: (usize, usize),
        pair: Pair,
        line_wise: bool,
    ) {
        self.record_boundary();
        let close_position = if line_wise {
            (end.0, end.1)
        } else {
            (end.0, end.1.saturating_add(1))
        };
        self.buffer
            .paste_before(close_position.0, close_position.1, &pair.close.to_string());
        self.buffer
            .paste_before(start.0, start.1, &pair.open.to_string());
        self.cursor.set_position(start.0, start.1);
    }
}

#[cfg(test)]
mod tests;
