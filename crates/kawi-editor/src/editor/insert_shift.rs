use super::session::InsertEdit;
use super::Editor;
use crate::editor::indent::{reindent_line, rounded_indent_width, IndentConfig};

impl Editor {
    pub(super) fn insert_shift_right(&mut self) {
        self.record_insert(InsertEdit::ShiftRight);
        self.insert_shift_indent(false, false);
    }

    pub(super) fn insert_shift_left(&mut self) {
        let collapse = self.collapses_indent();
        self.replay_insert_shift_left(collapse);
    }

    pub(super) fn replay_insert_shift_left(&mut self, collapse: bool) {
        self.record_insert(InsertEdit::ShiftLeft { collapse });
        self.insert_shift_indent(true, collapse);
    }

    fn collapses_indent(&self) -> bool {
        self.cursor.col() > 0 && matches!(self.insert_last_char, Some('0' | '^'))
    }

    fn insert_shift_indent(&mut self, left: bool, collapse: bool) {
        let row = self.cursor.row();
        let col = self.delete_collapsed_char(row, collapse);
        let line = self.buffer.line(row).unwrap_or_default();
        let indent = self.effective_indent();
        let before = ShiftState {
            col,
            width: kawi_buffer::indent_width_of(&line, indent.tabstop),
            ws_end: indent_end(&line),
            cursor_vcol: self.buffer.screen_col(row, col, indent.tabstop),
        };
        let new_width = shifted_width(before.width, left, collapse, &indent);
        self.apply_line_indent(row, &line, before.width, new_width, &indent);
        self.place_insert_cursor(row, &before, new_width, &indent);
    }

    fn delete_collapsed_char(&mut self, row: usize, collapse: bool) -> usize {
        let col = self.cursor.col();
        if !collapse {
            return col;
        }
        self.record_maybe();
        self.buffer.delete_range(row, col - 1, row, col - 1);
        self.cursor.set_col(col - 1);
        col - 1
    }

    fn apply_line_indent(
        &mut self,
        row: usize,
        line: &str,
        width: usize,
        new_width: usize,
        indent: &IndentConfig,
    ) {
        if new_width == width {
            return;
        }
        self.record_maybe();
        let next = reindent_line(line, new_width, indent.expandtab, indent.tabstop);
        self.buffer.replace_lines(row, row, &next);
    }

    fn place_insert_cursor(
        &mut self,
        row: usize,
        before: &ShiftState,
        new_width: usize,
        indent: &IndentConfig,
    ) {
        let line = self.buffer.line(row).unwrap_or_default();
        let new_ws_end = indent_end(&line);
        if before.col >= before.ws_end {
            self.cursor
                .set_position(row, new_ws_end + before.col - before.ws_end);
            return;
        }
        let target = shifted_vcol(before.cursor_vcol, before.width, new_width);
        let (index, reached) = self.reached_column(row, target, indent.tabstop);
        let pad = target - reached;
        if pad > 0 {
            self.record_maybe();
            for _ in 0..pad {
                self.buffer.insert_char(row, index, ' ');
            }
        }
        self.cursor.set_position(row, index + pad);
    }

    fn reached_column(&self, row: usize, target: usize, tabstop: usize) -> (usize, usize) {
        let len = self.buffer.line_len(row);
        let total = self.buffer.screen_col(row, len, tabstop);
        if target >= total {
            return (len, total);
        }
        let index = self.buffer.char_col(row, target, tabstop);
        (index, self.buffer.screen_col(row, index, tabstop))
    }
}

struct ShiftState {
    col: usize,
    width: usize,
    ws_end: usize,
    cursor_vcol: usize,
}

fn shifted_width(width: usize, left: bool, collapse: bool, indent: &IndentConfig) -> usize {
    if collapse {
        return 0;
    }
    rounded_indent_width(width, left, indent.shift_unit())
}

fn indent_end(line: &str) -> usize {
    line.chars()
        .position(|ch| !matches!(ch, ' ' | '\t'))
        .unwrap_or(line.chars().count())
}

fn shifted_vcol(cursor_vcol: usize, width: usize, new_width: usize) -> usize {
    if new_width >= width {
        cursor_vcol + (new_width - width)
    } else {
        cursor_vcol.saturating_sub(width - new_width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use crate::mode::Mode;
    use kawi_buffer::Buffer;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    fn open_insert(e: &mut Editor, row: usize, col: usize) {
        e.apply(Action::JumpTo(row, col));
        e.apply(Action::EnterInsert);
    }

    #[test]
    fn shift_right_inserts_tab() {
        let mut e = editor_with("ab");
        open_insert(&mut e, 0, 0);
        e.apply(Action::InsertShiftRight);
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "\tab");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn shift_right_on_empty_line_inserts_tab() {
        let mut e = editor_with("");
        open_insert(&mut e, 0, 0);
        e.apply(Action::InsertShiftRight);
        assert_eq!(e.text_lines()[0], "\t");
    }

    #[test]
    fn shift_left_removes_tab() {
        let mut e = editor_with("\tab");
        open_insert(&mut e, 0, 0);
        e.apply(Action::InsertShiftLeft);
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "ab");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn shift_right_rounds_to_shiftwidth_with_expandtab() {
        let mut e = editor_with("  ab");
        e.set_shiftwidth(4);
        e.set_expandtab(true);
        open_insert(&mut e, 0, 2);
        e.apply(Action::InsertShiftRight);
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "    ab");
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn shift_right_tabifies_mixed_indent() {
        let mut e = editor_with(" \tab");
        e.set_shiftwidth(4);
        open_insert(&mut e, 0, 0);
        e.apply(Action::InsertShiftRight);
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "    \t    ab");
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn shift_left_strips_remainder_before_shiftwidth() {
        let mut e = editor_with("        ");
        e.set_shiftwidth(4);
        open_insert(&mut e, 0, 4);
        e.apply(Action::InsertShiftLeft);
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "    ");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn shift_left_uses_tabstop_when_shiftwidth_is_zero() {
        let mut e = editor_with("\tab");
        e.set_shiftwidth(0);
        open_insert(&mut e, 0, 0);
        e.apply(Action::InsertShiftLeft);
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "ab");
    }

    #[test]
    fn zero_before_shift_left_collapses_indent() {
        let mut e = editor_with("  ab");
        open_insert(&mut e, 0, 2);
        e.apply(Action::InsertChar('0'));
        e.apply(Action::InsertShiftLeft);
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "ab");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn stale_last_char_skips_collapse() {
        let mut e = editor_with("ab");
        open_insert(&mut e, 0, 1);
        e.apply(Action::InsertChar('0'));
        e.apply(Action::MoveLeft);
        e.apply(Action::InsertShiftLeft);
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "a0b");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn counted_insert_replays_shift() {
        let mut e = editor_with("  ab");
        e.apply(Action::JumpTo(0, 2));
        e.apply(Action::PushCount(3));
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertShiftRight);
        e.apply(Action::InsertChar('x'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "\t\t\txxxab");
        assert_eq!((e.row(), e.col()), (0, 5));
        assert_eq!(e.mode(), Mode::Normal);
    }

    #[test]
    fn repeat_replays_shift() {
        let mut e = editor_with("ab\nzz");
        open_insert(&mut e, 0, 1);
        e.apply(Action::InsertShiftRight);
        e.apply(Action::InsertChar('x'));
        e.apply(Action::ExitInsert);
        e.apply(Action::MoveDown);
        e.apply(Action::RepeatLastChange);
        assert_eq!(e.text_lines(), vec!["\taxb", "\tzxz"]);
        assert_eq!((e.row(), e.col()), (1, 2));
    }

    #[test]
    fn repeat_replays_collapse() {
        let mut e = editor_with("  ab\nzz");
        open_insert(&mut e, 0, 1);
        e.apply(Action::InsertChar('0'));
        e.apply(Action::InsertShiftLeft);
        e.apply(Action::ExitInsert);
        e.apply(Action::MoveDown);
        e.apply(Action::RepeatLastChange);
        assert_eq!(e.text_lines(), vec!["ab", "zz"]);
        assert_eq!((e.row(), e.col()), (1, 0));
    }
}
