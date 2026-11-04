use super::session::InsertEdit;
use super::tab::TabExpansion;
use super::Editor;
use crate::mode::Mode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ReplaceEntry {
    Typed { original: Option<String> },
    Newline,
}

impl Editor {
    pub(super) fn enter_replace(&mut self) {
        self.buffer.begin_change();
        self.mode = Mode::Replace;
        self.insert_last_char = None;
        self.insert_start = Some((self.cursor.row(), self.cursor.col()));
        self.replace_entries.clear();
    }

    pub(super) fn insert_replace_char(&mut self, c: char) {
        self.record_maybe();
        self.record_insert(InsertEdit::Char(c));
        self.replace_at_cursor(c);
    }

    fn replace_at_cursor(&mut self, c: char) {
        let row = self.cursor.row();
        let col = self.cursor.col();
        let original = self.buffer.grapheme_text(row, col);
        self.replace_entries.push(ReplaceEntry::Typed { original });
        if self.buffer.char_at(row, col).is_some() {
            self.buffer.delete_range(row, col, row, col);
        }
        self.buffer.insert_char(row, col, c);
        self.cursor.advance_col();
    }

    fn insert_at_cursor(&mut self, c: char) {
        self.replace_entries
            .push(ReplaceEntry::Typed { original: None });
        let row = self.cursor.row();
        let col = self.cursor.col();
        self.buffer.insert_char(row, col, c);
        self.cursor.advance_col();
    }

    pub(super) fn mark_replace_newline(&mut self) {
        if self.mode == Mode::Replace {
            self.replace_entries.push(ReplaceEntry::Newline);
        }
    }

    pub(super) fn insert_replace_tab(&mut self) {
        let indent = self.effective_indent();
        let row = self.cursor.row();
        let col = self.cursor.col();
        let vcol = self.buffer.screen_col(row, col, indent.tabstop);
        match self.tab_expansion(row, col, vcol, &indent) {
            TabExpansion::Literal => {
                self.record_maybe();
                self.replace_at_cursor('\t');
            }
            TabExpansion::Spaces(width) => {
                if width == 0 {
                    return;
                }
                self.record_maybe();
                self.replace_at_cursor(' ');
                for _ in 1..width {
                    self.insert_at_cursor(' ');
                }
            }
        }
    }

    pub(super) fn replace_backspace(&mut self) {
        self.record_maybe();
        self.record_insert(InsertEdit::Backspace);
        let row = self.cursor.row();
        let col = self.cursor.col();
        if col == 0 {
            self.replace_erase_line_start(row);
            return;
        }
        self.cursor.set_position(row, col - 1);
        self.apply_replace_entry(row, col - 1);
    }

    pub(super) fn replace_backspace_word(&mut self) {
        self.record_maybe();
        self.record_insert(InsertEdit::BackspaceWord);
        let row = self.cursor.row();
        let col = self.cursor.col();
        if col == 0 {
            self.replace_erase_line_start(row);
            return;
        }
        let start = self.word_erase_start(row, col);
        self.erase_replace_span(row, start);
    }

    pub(super) fn replace_backspace_line(&mut self) {
        self.record_maybe();
        self.record_insert(InsertEdit::BackspaceLine);
        let row = self.cursor.row();
        let col = self.cursor.col();
        if col == 0 {
            self.replace_erase_line_start(row);
            return;
        }
        let start = self.insert_erase_stop(row, col, true);
        self.erase_replace_span(row, start);
    }

    fn erase_replace_span(&mut self, row: usize, start: usize) {
        while self.cursor.col() > start {
            let col = self.cursor.col() - 1;
            if !self.apply_replace_entry(row, col) {
                break;
            }
            self.cursor.set_position(row, col);
        }
    }

    fn apply_replace_entry(&mut self, row: usize, col: usize) -> bool {
        match self.replace_entries.last() {
            Some(ReplaceEntry::Typed {
                original: Some(text),
            }) => {
                let text = text.clone();
                self.replace_entries.pop();
                self.put_text(row, col, &text);
                true
            }
            Some(ReplaceEntry::Typed { original: None }) => {
                self.replace_entries.pop();
                self.buffer.delete_range(row, col, row, col);
                true
            }
            _ => false,
        }
    }

    fn replace_erase_line_start(&mut self, row: usize) {
        if row == 0 {
            return;
        }
        if let Some(ReplaceEntry::Newline) = self.replace_entries.last() {
            self.replace_entries.pop();
            let (join_row, join_col) = self.buffer.delete_backward(row, 0);
            self.cursor.set_position(join_row, join_col);
            return;
        }
        let previous = row - 1;
        self.cursor
            .set_position(previous, self.buffer.line_len(previous));
    }

    fn put_text(&mut self, row: usize, col: usize, text: &str) {
        self.buffer.delete_range(row, col, row, col);
        for (index, ch) in text.chars().enumerate() {
            self.buffer.insert_char(row, col + index, ch);
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

    fn enter_replace(e: &mut Editor, row: usize, col: usize) {
        e.apply(Action::JumpTo(row, col));
        e.apply(Action::EnterReplace);
    }

    #[test]
    fn enter_replace_switches_mode() {
        let mut e = editor_with("abc");
        e.apply(Action::EnterReplace);
        assert_eq!(e.mode(), Mode::Replace);
    }

    #[test]
    fn typing_replaces_chars_and_advances() {
        let mut e = editor_with("abcdef");
        enter_replace(&mut e, 0, 2);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::InsertChar('Y'));
        assert_eq!(e.text_lines()[0], "abXYef");
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn typing_past_eol_appends() {
        let mut e = editor_with("abcdef");
        enter_replace(&mut e, 0, 5);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::InsertChar('Y'));
        assert_eq!(e.text_lines()[0], "abcdeXY");
        assert_eq!((e.row(), e.col()), (0, 7));
    }

    #[test]
    fn backspace_restores_replaced_chars() {
        let mut e = editor_with("abcdef");
        enter_replace(&mut e, 0, 2);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::InsertChar('Y'));
        e.apply(Action::DeleteBackward);
        assert_eq!(e.text_lines()[0], "abXdef");
        assert_eq!((e.row(), e.col()), (0, 3));
        e.apply(Action::DeleteBackward);
        assert_eq!(e.text_lines()[0], "abcdef");
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn backspace_on_inserted_char_deletes_it() {
        let mut e = editor_with("abcdef");
        enter_replace(&mut e, 0, 5);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::InsertChar('Y'));
        e.apply(Action::DeleteBackward);
        assert_eq!(e.text_lines()[0], "abcdeX");
        assert_eq!((e.row(), e.col()), (0, 6));
        e.apply(Action::DeleteBackward);
        assert_eq!(e.text_lines()[0], "abcdef");
        assert_eq!((e.row(), e.col()), (0, 5));
    }

    #[test]
    fn backspace_with_empty_stack_only_moves() {
        let mut e = editor_with("abcdef");
        enter_replace(&mut e, 0, 2);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::DeleteBackward);
        e.apply(Action::DeleteBackward);
        assert_eq!(e.text_lines()[0], "abcdef");
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn enter_splits_without_replacing() {
        let mut e = editor_with("abcd");
        enter_replace(&mut e, 0, 1);
        e.apply(Action::InsertNewline);
        assert_eq!(e.text_lines(), vec!["a", "bcd"]);
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn backspace_joins_after_enter() {
        let mut e = editor_with("abcd");
        enter_replace(&mut e, 0, 1);
        e.apply(Action::InsertNewline);
        e.apply(Action::DeleteBackward);
        assert_eq!(e.text_lines(), vec!["abcd"]);
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn backspace_after_enter_restores_typed_char() {
        let mut e = editor_with("abcdef");
        enter_replace(&mut e, 0, 2);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::InsertNewline);
        e.apply(Action::InsertChar('Y'));
        e.apply(Action::DeleteBackward);
        e.apply(Action::DeleteBackward);
        e.apply(Action::DeleteBackward);
        assert_eq!(e.text_lines(), vec!["abcdef"]);
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn delete_forward_removes_char_without_touching_stack() {
        let mut e = editor_with("abcdef");
        enter_replace(&mut e, 0, 1);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::DeleteForward);
        assert_eq!(e.text_lines()[0], "aXdef");
        e.apply(Action::DeleteBackward);
        assert_eq!(e.text_lines()[0], "abdef");
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn count_repeats_replacement() {
        let mut e = editor_with("abcdef");
        e.apply(Action::PushCount(3));
        e.apply(Action::EnterReplace);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::InsertChar('Y'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "XYXYXY");
        assert_eq!((e.row(), e.col()), (0, 5));
    }

    #[test]
    fn exit_moves_cursor_left() {
        let mut e = editor_with("abcdef");
        enter_replace(&mut e, 0, 2);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!((e.row(), e.col()), (0, 2));
        assert_eq!(e.mode(), Mode::Normal);
    }

    #[test]
    fn undo_reverts_whole_session() {
        let mut e = editor_with("abcdef");
        enter_replace(&mut e, 0, 2);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::InsertChar('Y'));
        e.apply(Action::InsertNewline);
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines(), vec!["abcdef"]);
    }

    #[test]
    fn first_tab_in_replace_mode_is_undoable() {
        let mut e = editor_with("abcdefgh");
        enter_replace(&mut e, 0, 1);
        e.apply(Action::InsertTab);
        e.apply(Action::ExitInsert);
        assert!(e.is_dirty());
        e.apply(Action::Undo);
        assert_eq!(e.text_lines(), vec!["abcdefgh"]);
        assert!(!e.is_dirty());
    }

    #[test]
    fn tab_without_expandtab_replaces_one_char() {
        let mut e = editor_with("abcdefgh");
        enter_replace(&mut e, 0, 1);
        e.apply(Action::InsertTab);
        assert_eq!(e.text_lines()[0], "a\tcdefgh");
        e.apply(Action::DeleteBackward);
        assert_eq!(e.text_lines()[0], "abcdefgh");
    }

    #[test]
    fn expandtab_tab_replaces_one_char_and_inserts_rest() {
        let mut e = editor_with("abcdefgh");
        e.set_expandtab(true);
        enter_replace(&mut e, 0, 1);
        e.apply(Action::InsertTab);
        assert_eq!(e.text_lines()[0], "a       cdefgh");
        assert_eq!((e.row(), e.col()), (0, 8));
        for _ in 0..7 {
            e.apply(Action::DeleteBackward);
        }
        assert_eq!(e.text_lines()[0], "abcdefgh");
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn backspace_word_restores_replaced_chars() {
        let mut e = editor_with("one two three");
        enter_replace(&mut e, 0, 4);
        for ch in "AB CD".chars() {
            e.apply(Action::InsertChar(ch));
        }
        e.apply(Action::InsertBackspaceWord);
        assert_eq!(e.text_lines()[0], "one AB  three");
        e.apply(Action::InsertBackspaceWord);
        assert_eq!(e.text_lines()[0], "one two three");
    }

    #[test]
    fn backspace_line_restores_replaced_chars() {
        let mut e = editor_with("one two three");
        enter_replace(&mut e, 0, 4);
        for ch in "AB CD".chars() {
            e.apply(Action::InsertChar(ch));
        }
        e.apply(Action::InsertBackspaceLine);
        assert_eq!(e.text_lines()[0], "one two three");
        assert_eq!((e.row(), e.col()), (0, 4));
    }
}
