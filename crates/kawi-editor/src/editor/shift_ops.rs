use super::Editor;
use crate::actions::ShiftMotion;
use crate::editor::indent::{shift_span_text, IndentConfig, IndentShift};
use crate::find;
use crate::words;

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

    pub(super) fn shift_right_at(&mut self, motion: ShiftMotion) {
        self.apply_shift_motion(motion, Shift::Right);
    }

    pub(super) fn shift_left_at(&mut self, motion: ShiftMotion) {
        self.apply_shift_motion(motion, Shift::Left);
    }

    pub(super) fn reindent_at(&mut self, motion: ShiftMotion) {
        self.apply_shift_motion(motion, Shift::Reindent);
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
        self.apply_shift_rows(first, last, shift);
    }

    fn apply_shift_motion(&mut self, motion: ShiftMotion, shift: Shift) {
        let Some((first, last)) = self.shift_motion_rows(motion) else {
            return;
        };
        self.apply_shift_rows(first, last, shift);
    }

    fn apply_shift_rows(&mut self, first: usize, last: usize, shift: Shift) {
        let indent = self.effective_indent();
        let here = self
            .buffer
            .screen_col(self.cursor.row(), self.cursor.col(), indent.tabstop);
        let screen = here.max(self.cursor.desired());
        let text = self.buffer.lines_text(first, last);
        let shift_text = match shift {
            Shift::Right => shift_span_text(
                &text,
                IndentShift::In(shift_amount(&indent)),
                indent.expandtab,
                indent.tabstop,
            ),
            Shift::Left => shift_span_text(
                &text,
                IndentShift::Out(shift_amount(&indent)),
                indent.expandtab,
                indent.tabstop,
            ),
            Shift::Reindent => {
                shift_span_text(&text, IndentShift::Clear, indent.expandtab, indent.tabstop)
            }
        };
        let Some(shifted) = shift_text else {
            self.place_shift_cursor(first, screen, indent.tabstop);
            return;
        };
        self.record_boundary();
        self.buffer.replace_lines(first, last, &shifted);
        self.place_shift_cursor(first, screen, indent.tabstop);
    }

    fn shift_motion_rows(&mut self, motion: ShiftMotion) -> Option<(usize, usize)> {
        let (row, col) = (self.cursor.row(), self.cursor.col());
        match motion {
            ShiftMotion::WordForward => {
                let count = self.take_count();
                let span = words::dw_range(&self.buffer, row, col, count);
                span_rows(span)
            }
            ShiftMotion::WordBackward => {
                let count = self.take_count();
                let span = words::db_range(&self.buffer, row, col, count);
                span_rows(span)
            }
            ShiftMotion::WordEnd => {
                let count = self.take_count();
                let span = words::cw_range(&self.buffer, row, col, count);
                span_rows(span)
            }
            ShiftMotion::LineEnd => {
                let count = self.take_count();
                let tail = self.line_tail(count)?;
                self.remember_line_end();
                Some((tail.0, tail.2))
            }
            ShiftMotion::LineStart => {
                let (start, end) = self.line_start_span()?;
                Some((start.0, end.0))
            }
            ShiftMotion::Find {
                target,
                forward,
                till,
            } => {
                let count = self.take_count();
                let (start, end) = find::find_span(
                    &self.buffer,
                    (row, col),
                    target,
                    forward,
                    till,
                    count,
                    false,
                )?;
                Some((start.0, end.0))
            }
            ShiftMotion::RepeatFind {
                target,
                forward,
                till,
            } => {
                let count = self.take_count();
                let (start, end) =
                    find::find_span(&self.buffer, (row, col), target, forward, till, count, true)?;
                Some((start.0, end.0))
            }
            ShiftMotion::Object { inner, object } => {
                let count = self.take_count();
                let span = self.object_edit(inner, object, count)?;
                span_rows(span)
            }
            ShiftMotion::GotoLine { last } => {
                let target = self.goto_target(last);
                Some((row.min(target), row.max(target)))
            }
            ShiftMotion::Lines { upward } => {
                let count = self.take_count();
                self.counted_line_span(upward, count)
            }
        }
    }

    fn place_shift_cursor(&mut self, row: usize, screen: usize, tabstop: usize) {
        let row = row.min(self.buffer.line_count().saturating_sub(1));
        let col = self.buffer.char_col(row, screen, tabstop);
        self.cursor.set_position(row, col);
    }
}

#[derive(Clone, Copy)]
enum Shift {
    Right,
    Left,
    Reindent,
}

fn shift_amount(indent: &IndentConfig) -> usize {
    if indent.shiftwidth == 0 {
        indent.tabstop
    } else {
        indent.shiftwidth
    }
}

fn span_rows(span: words::WordSpan) -> Option<(usize, usize)> {
    match span {
        words::WordSpan::Charwise { start, end } => Some((start.0, end.0)),
        words::WordSpan::Linewise { first, last } => Some((first, last)),
        words::WordSpan::Empty { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{Action, Object};
    use crate::mode::Mode;
    use kawi_buffer::Buffer;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    fn editor_with_path(text: &str, path: &str) -> Editor {
        let mut buffer = Buffer::from_string(text);
        buffer.path = Some(path.to_string());
        Editor::new(buffer)
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

    #[test]
    fn shift_width_four_spaces() {
        let mut editor = editor_with("  foo");
        editor.set_shiftwidth(4);
        move_right(&mut editor, 2);
        editor.apply(Action::ShiftRight);
        assert_eq!(editor.text_lines()[0], "      foo");
        assert_eq!((editor.row(), editor.col()), (0, 2));
    }

    #[test]
    fn shift_width_four_tab_rebuild() {
        let mut editor = editor_with("\tfoo");
        editor.set_shiftwidth(4);
        move_right(&mut editor, 1);
        editor.apply(Action::ShiftRight);
        assert_eq!(editor.text_lines()[0], "\t    foo");
        assert_eq!((editor.row(), editor.col()), (0, 1));
    }

    #[test]
    fn shift_width_four_left() {
        let mut editor = editor_with("      foo");
        editor.set_shiftwidth(4);
        move_right(&mut editor, 6);
        editor.apply(Action::ShiftLeft);
        assert_eq!(editor.text_lines()[0], "  foo");
        assert_eq!((editor.row(), editor.col()), (0, 4));
    }

    #[test]
    fn shift_width_zero_follows_tabstop() {
        let mut editor = editor_with("foo");
        editor.set_shiftwidth(0);
        editor.apply(Action::ShiftRight);
        assert_eq!(editor.text_lines()[0], "\tfoo");
    }

    #[test]
    fn python_file_uses_four_spaces() {
        let mut editor = editor_with_path("foo", "x.py");
        editor.apply(Action::ShiftRight);
        assert_eq!(editor.text_lines()[0], "    foo");
    }

    #[test]
    fn go_file_uses_tabs() {
        let mut editor = editor_with_path("foo", "x.go");
        editor.apply(Action::ShiftRight);
        assert_eq!(editor.text_lines()[0], "\tfoo");
    }

    #[test]
    fn yaml_file_uses_two_spaces() {
        let mut editor = editor_with_path("k: v", "x.yaml");
        editor.apply(Action::ShiftRight);
        assert_eq!(editor.text_lines()[0], "  k: v");
    }

    #[test]
    fn scala_file_uses_two_spaces() {
        let mut editor = editor_with_path("x", "x.scala");
        editor.apply(Action::ShiftRight);
        assert_eq!(editor.text_lines()[0], "  x");
    }

    #[test]
    fn manual_shiftwidth_beats_filetype() {
        let mut editor = editor_with_path("foo", "x.py");
        editor.set_shiftwidth(2);
        editor.apply(Action::ShiftRight);
        assert_eq!(editor.text_lines()[0], "  foo");
    }

    #[test]
    fn manual_expandtab_beats_filetype() {
        let mut editor = editor_with_path("foo", "x.c");
        editor.set_expandtab(true);
        editor.apply(Action::ShiftRight);
        assert_eq!(editor.text_lines()[0], "        foo");
    }

    #[test]
    fn shift_word_forward() {
        let mut editor = editor_with("foo BAR baz");
        editor.apply(Action::ShiftRightAt {
            motion: ShiftMotion::WordForward,
        });
        assert_eq!(editor.text_lines()[0], "\tfoo BAR baz");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn shift_word_backward() {
        let mut editor = editor_with("    foo BAR baz");
        move_right(&mut editor, 10);
        editor.apply(Action::ShiftLeftAt {
            motion: ShiftMotion::WordBackward,
        });
        assert_eq!(editor.text_lines()[0], "foo BAR baz");
        assert_eq!((editor.row(), editor.col()), (0, 10));
    }

    #[test]
    fn shift_line_end_max_column() {
        let mut editor = editor_with("foo BAR");
        editor.apply(Action::ShiftRightAt {
            motion: ShiftMotion::LineEnd,
        });
        assert_eq!(editor.text_lines()[0], "\tfoo BAR");
        assert_eq!((editor.row(), editor.col()), (0, 7));
    }

    #[test]
    fn shift_line_start() {
        let mut editor = editor_with("    foo");
        move_right(&mut editor, 4);
        editor.apply(Action::ReindentAt {
            motion: ShiftMotion::LineStart,
        });
        assert_eq!(editor.text_lines()[0], "foo");
        assert_eq!((editor.row(), editor.col()), (0, 2));
    }

    #[test]
    fn shift_find_forward() {
        let mut editor = editor_with("axb foo");
        editor.apply(Action::ShiftRightAt {
            motion: ShiftMotion::Find {
                target: 'x',
                forward: true,
                till: false,
            },
        });
        assert_eq!(editor.text_lines()[0], "\taxb foo");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn shift_repeat_find() {
        let mut editor = editor_with("axbxc");
        editor.apply(Action::MoveRight);
        editor.apply(Action::ShiftRightAt {
            motion: ShiftMotion::RepeatFind {
                target: 'x',
                forward: true,
                till: false,
            },
        });
        assert_eq!(editor.text_lines()[0], "\taxbxc");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn shift_paren_object() {
        let mut editor = editor_with("foo (bar) baz");
        move_right(&mut editor, 5);
        editor.apply(Action::ShiftRightAt {
            motion: ShiftMotion::Object {
                inner: true,
                object: Object::Paren,
            },
        });
        assert_eq!(editor.text_lines()[0], "\tfoo (bar) baz");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn shift_goto_last_line() {
        let mut editor = editor_with("a\nb\nc");
        editor.apply(Action::ShiftRightAt {
            motion: ShiftMotion::GotoLine { last: true },
        });
        assert_eq!(editor.text_lines()[0], "\ta");
        assert_eq!(editor.text_lines()[1], "\tb");
        assert_eq!(editor.text_lines()[2], "\tc");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn shift_goto_first_line() {
        let mut editor = editor_with("  a\n  b\n  c");
        editor.apply(Action::MoveDown);
        editor.apply(Action::MoveDown);
        editor.apply(Action::ShiftLeftAt {
            motion: ShiftMotion::GotoLine { last: false },
        });
        assert_eq!(editor.text_lines()[0], "a");
        assert_eq!(editor.text_lines()[1], "b");
        assert_eq!(editor.text_lines()[2], "c");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn shift_lines_down_count() {
        let mut editor = editor_with("a\nb\nc\nd");
        push_digits(&mut editor, &[2]);
        editor.apply(Action::ShiftRightAt {
            motion: ShiftMotion::Lines { upward: false },
        });
        assert_eq!(editor.text_lines()[0], "\ta");
        assert_eq!(editor.text_lines()[1], "\tb");
        assert_eq!(editor.text_lines()[2], "\tc");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn shift_lines_up_at_top_is_noop() {
        let mut editor = editor_with("a\nb");
        editor.apply(Action::ShiftRightAt {
            motion: ShiftMotion::Lines { upward: true },
        });
        assert_eq!(editor.text_lines()[0], "a");
        assert_eq!((editor.row(), editor.col()), (0, 0));
        assert!(!editor.is_dirty());
    }

    #[test]
    fn shift_motion_undoes_together() {
        let mut editor = editor_with("a\nb\nc");
        editor.apply(Action::ShiftRightAt {
            motion: ShiftMotion::GotoLine { last: true },
        });
        editor.apply(Action::Undo);
        assert_eq!(editor.text_lines()[0], "a");
        assert_eq!(editor.text_lines()[1], "b");
        assert_eq!(editor.text_lines()[2], "c");
    }

    #[test]
    fn shift_motion_leaves_register() {
        let mut editor = editor_with("foo");
        editor.apply(Action::ShiftRightAt {
            motion: ShiftMotion::WordForward,
        });
        assert_eq!(editor.register_text(), String::new());
    }

    #[test]
    fn shift_many_lines_single_splice() {
        let text = (0..2000)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut editor = editor_with(&text);
        push_digits(&mut editor, &[2, 0, 0, 0]);
        editor.apply(Action::ShiftRight);
        assert_eq!(editor.line_count(), 2000);
        assert_eq!(editor.text_lines()[0], "\tline 0");
        assert_eq!(editor.text_lines()[1999], "\tline 1999");
        editor.apply(Action::Undo);
        assert_eq!(editor.text_lines()[0], "line 0");
    }
}
