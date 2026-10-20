use super::session::InsertEdit;
use super::Editor;
use crate::words::WordClass;

impl Editor {
    pub(super) fn insert_backspace_word(&mut self) {
        if self.mode == crate::mode::Mode::Replace {
            self.replace_backspace_word();
            return;
        }
        let Some((row, col)) = self.insert_erase_start() else {
            return;
        };
        self.record_maybe();
        self.record_insert(InsertEdit::BackspaceWord);
        if col == 0 {
            self.join_insert_line(row);
            return;
        }
        let start = self.word_erase_start(row, col);
        if start < col {
            self.buffer.delete_range(row, start, row, col - 1);
        }
        self.cursor.set_position(row, start);
    }

    pub(super) fn word_erase_start(&self, row: usize, col: usize) -> usize {
        let stop = self.insert_erase_stop(row, col, false);
        let mut col = col;
        let mut class = self.buffer_class(row, col);
        let mut word_char = false;
        let mut in_word = false;
        loop {
            col = col.saturating_sub(1);
            let current = self.buffer_class(row, col);
            let previous = class;
            class = current;
            if !in_word && current != WordClass::Space {
                in_word = true;
                word_char = current == WordClass::Word;
            } else if in_word
                && (current == WordClass::Space
                    || (current == WordClass::Word) != word_char
                    || previous != current)
            {
                col = col.saturating_add(1);
                break;
            }
            if col == 0 || col == stop {
                break;
            }
        }
        col
    }

    pub(super) fn insert_backspace_line(&mut self) {
        if self.mode == crate::mode::Mode::Replace {
            self.replace_backspace_line();
            return;
        }
        let Some((row, col)) = self.insert_erase_start() else {
            return;
        };
        self.record_maybe();
        self.record_insert(InsertEdit::BackspaceLine);
        if col == 0 {
            self.join_insert_line(row);
            return;
        }
        let stop = self.insert_erase_stop(row, col, true);
        if stop < col {
            self.buffer
                .delete_range(row, stop, row, col.saturating_sub(1));
        }
        self.cursor.set_position(row, stop);
    }

    fn insert_erase_start(&self) -> Option<(usize, usize)> {
        if self.buffer.is_empty() {
            return None;
        }
        let row = self.cursor.row();
        let col = self.cursor.col();
        if row == 0 && col == 0 {
            return None;
        }
        Some((row, col))
    }

    pub(super) fn insert_erase_stop(&self, row: usize, col: usize, keep_indent: bool) -> usize {
        let floor = if keep_indent {
            self.first_non_blank_of(row).min(col)
        } else {
            0
        };
        match self.insert_start {
            Some((start_row, start_col)) if start_row == row => start_col.max(floor).min(col),
            _ => floor,
        }
    }

    fn join_insert_line(&mut self, row: usize) {
        let (row, col) = self.buffer.delete_backward(row, 0);
        self.cursor.set_position(row, col);
        if let Some((start_row, _)) = self.insert_start {
            let start_row = start_row.saturating_sub(1);
            self.insert_start = Some((start_row, self.buffer.line_len(start_row)));
        }
    }

    fn buffer_class(&self, row: usize, col: usize) -> WordClass {
        match self.buffer.char_at(row, col) {
            Some(ch) if ch.is_alphanumeric() || ch == '_' => WordClass::Word,
            Some(ch) if ch.is_whitespace() => WordClass::Space,
            Some(_) => WordClass::Punct,
            None => WordClass::Space,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use kawi_buffer::Buffer;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    fn open_insert(e: &mut Editor, row: usize, col: usize) {
        e.apply(Action::JumpTo(row, col));
        e.apply(Action::EnterInsert);
    }

    #[test]
    fn ctrl_w_deletes_word_before_cursor() {
        let mut e = editor_with("foo bar baz");
        open_insert(&mut e, 0, 8);
        e.apply(Action::InsertBackspaceWord);
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "foo baz");
    }

    #[test]
    fn ctrl_w_stops_at_insert_start() {
        let mut e = editor_with("foo bar baz");
        e.apply(Action::JumpTo(0, 5));
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::InsertChar('Y'));
        e.apply(Action::InsertBackspaceWord);
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "foo bar baz");
    }

    #[test]
    fn ctrl_w_at_line_start_joins_previous() {
        let mut e = editor_with("abc\ndef");
        open_insert(&mut e, 1, 0);
        e.apply(Action::InsertBackspaceWord);
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines(), vec!["abcdef"]);
    }

    #[test]
    fn ctrl_u_deletes_to_insert_start() {
        let mut e = editor_with("abcdef");
        e.apply(Action::JumpTo(0, 3));
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::InsertChar('Y'));
        e.apply(Action::InsertBackspaceLine);
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "abcdef");
        assert_eq!(e.row(), 0);
    }

    #[test]
    fn ctrl_u_keeps_indent() {
        let mut e = editor_with("  abc");
        open_insert(&mut e, 0, 4);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::InsertChar('Y'));
        e.apply(Action::InsertBackspaceLine);
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "  abc");
    }

    #[test]
    fn ctrl_u_at_line_start_joins_previous() {
        let mut e = editor_with("abc\ndef");
        open_insert(&mut e, 1, 0);
        e.apply(Action::InsertBackspaceLine);
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines(), vec!["abcdef"]);
    }
}
