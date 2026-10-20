use super::repeat::RepeatChange;
use super::session::InsertEdit;
use super::Editor;
use crate::mode::Mode;

impl Editor {
    pub fn paste(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.clear_count();
        match self.mode {
            Mode::Normal => self.paste_normal(text),
            Mode::Insert | Mode::Replace => self.paste_insert(text),
            Mode::Command | Mode::Search => self.paste_prompt(text),
            Mode::Visual | Mode::VisualLine | Mode::VisualBlock => self.paste_visual(text),
        }
        self.apply_line_adjustments();
    }

    pub(super) fn paste_normal(&mut self, text: &str) {
        self.record_boundary();
        let (row, col) = (self.cursor.row(), self.cursor.col());
        let target = col.saturating_add(1).min(self.buffer.line_len(row));
        let end = pasted_end(row, target, text);
        self.buffer.paste_after(row, col, text);
        self.place_paste_cursor(end, text);
        self.record_change(RepeatChange::Paste {
            text: text.to_string(),
        });
    }

    fn paste_insert(&mut self, text: &str) {
        self.record_insert(InsertEdit::Text(text.to_string()));
        self.insert_pasted_text(text);
    }

    fn paste_visual(&mut self, text: &str) {
        let line_wise = self.mode == Mode::VisualLine;
        let ((start_row, start_col), _) = self.operator_range();
        self.delete_selection();
        if self.mode.is_visual() {
            self.exit_visual();
        }
        let (row, col) = if line_wise && self.cursor.row() < start_row {
            let row = self
                .buffer
                .paste_lines_below(self.cursor.row(), &[String::new()]);
            (row, 0)
        } else {
            (start_row, start_col)
        };
        let end = pasted_end(row, col, text);
        self.buffer.paste_before(row, col, text);
        self.place_paste_cursor(end, text);
    }

    fn paste_prompt(&mut self, text: &str) {
        let first = text.split('\n').next().unwrap_or(text);
        for ch in first.chars() {
            match self.mode {
                Mode::Command => self.push_command_char(ch),
                Mode::Search => self.push_search_char(ch),
                Mode::Normal
                | Mode::Insert
                | Mode::Replace
                | Mode::Visual
                | Mode::VisualLine
                | Mode::VisualBlock => {
                    return;
                }
            }
        }
    }

    pub(super) fn insert_pasted_text(&mut self, text: &str) {
        self.record_maybe();
        let (row, col) = (self.cursor.row(), self.cursor.col());
        let end = pasted_end(row, col, text);
        self.buffer.paste_before(row, col, text);
        self.cursor.set_position(end.0, end.1);
    }

    fn place_paste_cursor(&mut self, end: (usize, usize), text: &str) {
        let (row, col) = last_pasted_cell(end, text);
        self.cursor.set_position(row, col);
        self.remember_wanted_column();
    }
}

fn pasted_end(row: usize, col: usize, text: &str) -> (usize, usize) {
    text.chars().fold((row, col), |(row, col), ch| {
        if ch == '\n' {
            (row + 1, 0)
        } else {
            (row, col + 1)
        }
    })
}

fn last_pasted_cell((row, col): (usize, usize), text: &str) -> (usize, usize) {
    if text.ends_with('\n') {
        (row, col)
    } else {
        (row, col.saturating_sub(1))
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

    fn with_cursor(text: &str, row: usize, col: usize) -> Editor {
        let mut e = editor_with(text);
        e.cursor.set_position(row, col);
        e
    }

    #[test]
    fn normal_paste_inserts_after_cursor() {
        let mut e = with_cursor("hello world\nsecond\n", 1, 4);
        e.paste("abc");
        assert_eq!(e.text_lines(), vec!["hello world", "seconabcd"]);
        assert_eq!((e.row(), e.col()), (1, 7));
    }

    #[test]
    fn normal_paste_trailing_newline_lands_on_next_line() {
        let mut e = with_cursor("hello world\nsecond\n", 1, 4);
        e.paste("abc\n");
        assert_eq!(e.text_lines(), vec!["hello world", "seconabc", "d"]);
        assert_eq!((e.row(), e.col()), (2, 0));
    }

    #[test]
    fn normal_paste_multiline_splits_lines() {
        let mut e = with_cursor("hello world\nsecond\n", 1, 4);
        e.paste("x\ny");
        assert_eq!(e.text_lines(), vec!["hello world", "seconx", "yd"]);
        assert_eq!((e.row(), e.col()), (2, 0));
    }

    #[test]
    fn normal_paste_at_line_end_appends() {
        let mut e = with_cursor("ab\ncd\n", 1, 1);
        e.paste("abc");
        assert_eq!(e.text_lines(), vec!["ab", "cdabc"]);
        assert_eq!((e.row(), e.col()), (1, 4));
    }

    #[test]
    fn normal_paste_is_one_undo_block() {
        let mut e = with_cursor("ab\ncd\n", 1, 0);
        e.paste("x\ny\n");
        assert_eq!(e.text_lines(), vec!["ab", "cx", "y", "d"]);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines(), vec!["ab", "cd"]);
    }

    #[test]
    fn normal_paste_repeats_with_dot() {
        let mut e = with_cursor("ab\n", 0, 1);
        e.paste("x");
        assert_eq!(e.text_lines(), vec!["abx"]);
        e.apply(Action::RepeatLastChange);
        assert_eq!(e.text_lines(), vec!["abxx"]);
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn normal_paste_keeps_unnamed_register() {
        let mut e = with_cursor("keep\nline2\n", 0, 0);
        e.apply(Action::YankLine);
        e.paste("zzz");
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines(), vec!["kzzzeep", "keep", "line2"]);
    }

    #[test]
    fn insert_paste_inserts_at_cursor() {
        let mut e = with_cursor("hello world\nsecond\n", 1, 4);
        e.mode = Mode::Insert;
        e.paste("abc");
        assert_eq!(e.text_lines(), vec!["hello world", "secoabcnd"]);
        assert_eq!((e.row(), e.col()), (1, 7));
    }

    #[test]
    fn insert_paste_multiline_inserts_newlines() {
        let mut e = with_cursor("hello world\nsecond\n", 1, 4);
        e.mode = Mode::Insert;
        e.paste("x\ny");
        assert_eq!(e.text_lines(), vec!["hello world", "secox", "ynd"]);
        assert_eq!((e.row(), e.col()), (2, 1));
    }

    #[test]
    fn insert_paste_repeats_with_dot() {
        let mut e = with_cursor("ab\n", 0, 0);
        e.apply(Action::EnterInsert);
        e.paste("xy");
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines(), vec!["xyab"]);
        e.apply(Action::RepeatLastChange);
        assert_eq!(e.text_lines(), vec!["xxyyab"]);
    }

    #[test]
    fn visual_line_paste_replaces_lines() {
        let mut e = with_cursor("one\ntwo\nthree\n", 0, 0);
        e.apply(Action::EnterVisualLine);
        e.apply(Action::MoveDown);
        e.paste("NEW\n");
        assert_eq!(e.text_lines(), vec!["NEW", "three"]);
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn visual_line_paste_without_newline_keeps_following_text() {
        let mut e = with_cursor("one\ntwo\nthree\n", 0, 0);
        e.apply(Action::EnterVisualLine);
        e.apply(Action::MoveDown);
        e.paste("NEW");
        assert_eq!(e.text_lines(), vec!["NEWthree"]);
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn visual_char_paste_replaces_selection() {
        let mut e = with_cursor("one\ntwo\nthree\n", 0, 0);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveDown);
        e.paste("NEW\n");
        assert_eq!(e.text_lines(), vec!["NEW", "wo", "three"]);
    }

    #[test]
    fn paste_clears_pending_count() {
        let mut e = with_cursor("ab\n", 0, 0);
        e.apply(Action::PushCount(3));
        e.paste("x");
        assert_eq!(e.text_lines(), vec!["axb"]);
    }

    #[test]
    fn normal_paste_shifts_marks_below() {
        let mut e = with_cursor("one\ntwo\nthree\nfour\n", 3, 0);
        e.apply(Action::SetMark('a'));
        e.cursor.set_position(0, 0);
        e.paste("x\ny\n");
        e.apply(Action::JumpToMark {
            name: 'a',
            line_wise: true,
        });
        assert_eq!(e.row(), 5);
    }

    #[test]
    fn command_paste_inserts_first_line() {
        let mut e = editor_with("one\ntwo\n");
        e.apply(Action::EnterCommand);
        e.paste("e file\ntwo\n");
        assert_eq!(e.command_text(), "e file");
    }

    #[test]
    fn search_paste_inserts_first_line() {
        let mut e = editor_with("one\ntwo\n");
        e.apply(Action::EnterSearch);
        e.paste("tw\no\n");
        assert_eq!(e.search_pattern(), "tw");
    }
}
