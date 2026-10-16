use super::Editor;

const SHIFT_WIDTH: usize = 8;
const TAB_STOP: usize = 8;

#[derive(Clone, Copy)]
enum Shift {
    Right,
    Left,
    Reindent,
}

impl Editor {
    pub(super) fn shift_right(&mut self) {
        self.apply_shift(Shift::Right);
    }

    pub(super) fn shift_left(&mut self) {
        self.apply_shift(Shift::Left);
    }

    pub(super) fn reindent(&mut self) {
        self.apply_shift(Shift::Reindent);
    }

    fn apply_shift(&mut self, shift: Shift) {
        let (first, last) = if self.mode.is_visual() {
            self.touched_rows()
        } else {
            let count = self.take_count();
            let row = self.cursor.row();
            let Some(last) = self.counted_last_row(count) else {
                return;
            };
            (row, last)
        };
        let screen = self
            .buffer
            .screen_col(self.cursor.row(), self.cursor.col(), TAB_STOP);
        if self.shift_affects(first, last, shift) {
            self.record_boundary();
            for row in first..=last {
                match shift {
                    Shift::Right => self.shift_row(row, true),
                    Shift::Left => self.shift_row(row, false),
                    Shift::Reindent => self.buffer.set_indent(row, ""),
                }
            }
        }
        self.place_shift_cursor(first, screen);
    }

    fn shift_affects(&self, first: usize, last: usize, shift: Shift) -> bool {
        (first..=last).any(|row| match shift {
            Shift::Right => self.buffer.line_len(row) != 0,
            Shift::Left => self.buffer.indent_width(row, TAB_STOP) != 0,
            Shift::Reindent => self.buffer.indent_end(row) != 0,
        })
    }

    fn shift_row(&mut self, row: usize, right: bool) {
        if self.buffer.line_len(row) == 0 {
            return;
        }
        let width = self.buffer.indent_width(row, TAB_STOP);
        let indented = if right {
            width + SHIFT_WIDTH
        } else {
            width.saturating_sub(SHIFT_WIDTH)
        };
        self.buffer.set_indent(row, &canonical_indent(indented));
    }

    fn place_shift_cursor(&mut self, row: usize, screen: usize) {
        let row = row.min(self.buffer.line_count().saturating_sub(1));
        let col = self.buffer.char_col(row, screen, TAB_STOP);
        self.cursor.set_position(row, col);
    }
}

fn canonical_indent(width: usize) -> String {
    "\t".repeat(width / TAB_STOP) + &" ".repeat(width % TAB_STOP)
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

    fn push_digits(editor: &mut Editor, digits: &[u8]) {
        for digit in digits {
            editor.apply(Action::PushCount(*digit));
        }
    }

    fn move_right(editor: &mut Editor, count: usize) {
        for _ in 0..count {
            editor.apply(Action::MoveRight);
        }
    }

    #[test]
    fn shift_right_basic() {
        let mut editor = editor_with("foo");
        editor.apply(Action::ShiftRight);
        assert_eq!(editor.text_lines()[0], "\tfoo");
        assert_eq!((editor.row(), editor.col()), (0, 0));
        assert_eq!(editor.register_text(), String::new());
    }

    #[test]
    fn shift_right_indented() {
        let mut editor = editor_with("  foo");
        move_right(&mut editor, 2);
        editor.apply(Action::ShiftRight);
        assert_eq!(editor.text_lines()[0], "\t  foo");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn shift_right_count() {
        let mut editor = editor_with("a\nb\nc");
        push_digits(&mut editor, &[2]);
        editor.apply(Action::ShiftRight);
        assert_eq!(editor.text_lines()[0], "\ta");
        assert_eq!(editor.text_lines()[1], "\tb");
        assert_eq!(editor.text_lines()[2], "c");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn shift_right_empty_noop() {
        let mut editor = editor_with("\nb");
        editor.apply(Action::ShiftRight);
        assert_eq!(editor.text_lines()[0], "");
        assert_eq!((editor.row(), editor.col()), (0, 0));
        assert!(!editor.is_dirty());
    }

    #[test]
    fn shift_right_whitespace_only() {
        let mut editor = editor_with("   ");
        editor.apply(Action::ShiftRight);
        assert_eq!(editor.text_lines()[0], "\t   ");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn shift_right_tab_indent() {
        let mut editor = editor_with("\tfoo");
        move_right(&mut editor, 1);
        editor.apply(Action::ShiftRight);
        assert_eq!(editor.text_lines()[0], "\t\tfoo");
        assert_eq!((editor.row(), editor.col()), (0, 1));
    }

    #[test]
    fn shift_right_wide_indent_goes_to_zero() {
        let mut editor = editor_with("          abcdef");
        move_right(&mut editor, 5);
        editor.apply(Action::ShiftRight);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn shift_left_basic() {
        let mut editor = editor_with("        foo");
        move_right(&mut editor, 8);
        editor.apply(Action::ShiftLeft);
        assert_eq!(editor.text_lines()[0], "foo");
        assert_eq!((editor.row(), editor.col()), (0, 2));
    }

    #[test]
    fn shift_left_partial() {
        let mut editor = editor_with("    foo");
        move_right(&mut editor, 4);
        editor.apply(Action::ShiftLeft);
        assert_eq!(editor.text_lines()[0], "foo");
        assert_eq!((editor.row(), editor.col()), (0, 2));
    }

    #[test]
    fn shift_left_tab() {
        let mut editor = editor_with("\tfoo");
        move_right(&mut editor, 1);
        editor.apply(Action::ShiftLeft);
        assert_eq!(editor.text_lines()[0], "foo");
        assert_eq!((editor.row(), editor.col()), (0, 2));
    }

    #[test]
    fn shift_left_count() {
        let mut editor = editor_with("a\n  b\nc");
        push_digits(&mut editor, &[2]);
        editor.apply(Action::ShiftLeft);
        assert_eq!(editor.text_lines()[0], "a");
        assert_eq!(editor.text_lines()[1], "b");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn shift_left_at_zero_noop() {
        let mut editor = editor_with("foo");
        editor.apply(Action::ShiftLeft);
        assert_eq!(editor.text_lines()[0], "foo");
        assert_eq!((editor.row(), editor.col()), (0, 0));
        assert!(!editor.is_dirty());
    }

    #[test]
    fn shift_left_sixteen_spaces() {
        let mut editor = editor_with("                foo");
        editor.apply(Action::ShiftLeft);
        assert_eq!(editor.text_lines()[0], "\tfoo");
    }

    #[test]
    fn reindent_strips_spaces() {
        let mut editor = editor_with("    foo");
        move_right(&mut editor, 4);
        editor.apply(Action::Reindent);
        assert_eq!(editor.text_lines()[0], "foo");
        assert_eq!((editor.row(), editor.col()), (0, 2));
    }

    #[test]
    fn reindent_strips_tabs() {
        let mut editor = editor_with("\tfoo");
        move_right(&mut editor, 1);
        editor.apply(Action::Reindent);
        assert_eq!(editor.text_lines()[0], "foo");
        assert_eq!((editor.row(), editor.col()), (0, 2));
    }

    #[test]
    fn reindent_empty_noop() {
        let mut editor = editor_with("\nb");
        editor.apply(Action::Reindent);
        assert_eq!(editor.text_lines()[0], "");
        assert!(!editor.is_dirty());
    }

    #[test]
    fn reindent_count() {
        let mut editor = editor_with("    a\n    b");
        push_digits(&mut editor, &[2]);
        editor.apply(Action::Reindent);
        assert_eq!(editor.text_lines()[0], "a");
        assert_eq!(editor.text_lines()[1], "b");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn visual_shift_right() {
        let mut editor = editor_with("ab\ncd");
        editor.apply(Action::MoveRight);
        editor.apply(Action::EnterVisual);
        editor.apply(Action::MoveDown);
        editor.apply(Action::MoveRight);
        editor.apply(Action::ShiftRight);
        assert_eq!(editor.text_lines()[0], "\tab");
        assert_eq!(editor.text_lines()[1], "\tcd");
        assert_eq!(editor.mode(), Mode::Visual);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn visual_shift_left() {
        let mut editor = editor_with("        ab\n        cd");
        editor.apply(Action::EnterVisualLine);
        editor.apply(Action::MoveDown);
        editor.apply(Action::ShiftLeft);
        assert_eq!(editor.text_lines()[0], "ab");
        assert_eq!(editor.text_lines()[1], "cd");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn visual_shift_left_screen_column() {
        let mut editor = editor_with("        ab\n        cd");
        move_right(&mut editor, 5);
        editor.apply(Action::EnterVisualLine);
        editor.apply(Action::MoveDown);
        editor.apply(Action::ShiftLeft);
        assert_eq!((editor.row(), editor.col()), (0, 1));
    }

    #[test]
    fn visual_reindent() {
        let mut editor = editor_with("    a\n    b");
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::EnterVisual);
        editor.apply(Action::MoveDown);
        editor.apply(Action::Reindent);
        assert_eq!(editor.text_lines()[0], "a");
        assert_eq!(editor.text_lines()[1], "b");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn shift_undoes_together() {
        let mut editor = editor_with("a\nb");
        push_digits(&mut editor, &[2]);
        editor.apply(Action::ShiftRight);
        editor.apply(Action::Undo);
        assert_eq!(editor.text_lines()[0], "a");
        assert_eq!(editor.text_lines()[1], "b");
    }
}
