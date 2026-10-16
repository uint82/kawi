use super::Editor;
use crate::actions::CaseMotion;
use crate::find;
use crate::words;

impl Editor {
    pub(super) fn toggle_case(&mut self) {
        self.shift_selection_case(toggle_ascii_case);
    }

    pub(super) fn lowercase(&mut self) {
        self.shift_selection_case(lower_ascii_case);
    }

    pub(super) fn uppercase(&mut self) {
        self.shift_selection_case(upper_ascii_case);
    }

    pub(super) fn toggle_case_at(&mut self, motion: CaseMotion) {
        self.shift_case_motion(motion, toggle_ascii_case);
    }

    pub(super) fn lowercase_at(&mut self, motion: CaseMotion) {
        self.shift_case_motion(motion, lower_ascii_case);
    }

    pub(super) fn uppercase_at(&mut self, motion: CaseMotion) {
        self.shift_case_motion(motion, upper_ascii_case);
    }

    fn shift_selection_case(&mut self, shift: fn(char) -> char) {
        self.snapshot_visual();
        let (start, end) = self.operator_range();
        self.record_boundary();
        self.buffer
            .shift_case_range(start.0, start.1, end.0, end.1, shift);
        self.collapse_to(start.0, start.1);
        self.cursor.set_position(start.0, start.1);
    }

    fn shift_case_motion(&mut self, motion: CaseMotion, shift: fn(char) -> char) {
        let Some(span) = self.case_motion_span(motion) else {
            return;
        };
        self.apply_case_span(span, shift);
    }

    fn case_motion_span(&mut self, motion: CaseMotion) -> Option<words::WordSpan> {
        let (row, col) = (self.cursor.row(), self.cursor.col());
        match motion {
            CaseMotion::WordForward => {
                let count = self.take_count();
                Some(words::dw_range(&self.buffer, row, col, count))
            }
            CaseMotion::WordBackward => {
                let count = self.take_count();
                Some(words::db_range(&self.buffer, row, col, count))
            }
            CaseMotion::WordEnd => {
                let count = self.take_count();
                Some(words::cw_range(&self.buffer, row, col, count))
            }
            CaseMotion::LineEnd => {
                let count = self.take_count();
                let tail = self.line_tail(count)?;
                Some(words::WordSpan::Charwise {
                    start: (tail.0, tail.1),
                    end: (tail.2, tail.3),
                })
            }
            CaseMotion::LineStart => {
                let (start, end) = self.line_start_span()?;
                Some(words::WordSpan::Charwise { start, end })
            }
            CaseMotion::Find {
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
                Some(words::WordSpan::Charwise { start, end })
            }
            CaseMotion::RepeatFind {
                target,
                forward,
                till,
            } => {
                let count = self.take_count();
                let (start, end) =
                    find::find_span(&self.buffer, (row, col), target, forward, till, count, true)?;
                Some(words::WordSpan::Charwise { start, end })
            }
            CaseMotion::Object { inner, object } => {
                let count = self.take_count();
                self.object_edit(inner, object, count)
            }
            CaseMotion::GotoLine { last } => {
                let row = self.cursor.row();
                let target = self.goto_target(last);
                let (first, last) = (row.min(target), row.max(target));
                Some(words::WordSpan::Linewise { first, last })
            }
            CaseMotion::Lines { upward } => {
                let count = self.take_count();
                let (first, last) = self.counted_line_span(upward, count)?;
                Some(words::WordSpan::Linewise { first, last })
            }
        }
    }

    fn apply_case_span(&mut self, span: words::WordSpan, shift: fn(char) -> char) {
        match span {
            words::WordSpan::Charwise { start, end } => {
                self.record_boundary();
                self.buffer
                    .shift_case_range(start.0, start.1, end.0, end.1, shift);
                self.place_case_cursor(start);
            }
            words::WordSpan::Linewise { first, last } => {
                self.record_boundary();
                for row in first..=last {
                    if self.buffer.line_len(row) == 0 {
                        continue;
                    }
                    let end = self.buffer.line_len(row).saturating_sub(1);
                    self.buffer.shift_case_range(row, 0, row, end, shift);
                }
                self.place_case_cursor((first, self.first_non_blank_of(first)));
            }
            words::WordSpan::Empty { .. } => {}
        }
    }

    fn place_case_cursor(&mut self, pos: (usize, usize)) {
        self.selection = None;
        self.cursor.set_position(pos.0, pos.1);
    }
}

// TODO(case): match vim towupper for non-ASCII text.
fn toggle_ascii_case(ch: char) -> char {
    if ch.is_ascii_lowercase() {
        ch.to_ascii_uppercase()
    } else if ch.is_ascii_uppercase() {
        ch.to_ascii_lowercase()
    } else {
        ch
    }
}

fn lower_ascii_case(ch: char) -> char {
    ch.to_ascii_lowercase()
}

fn upper_ascii_case(ch: char) -> char {
    ch.to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{Action, Object};
    use kawi_buffer::Buffer;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    fn push_digits(editor: &mut Editor, digits: &[u8]) {
        for digit in digits {
            editor.apply(Action::PushCount(*digit));
        }
    }

    #[test]
    fn toggle_word_forward() {
        let mut editor = editor_with("foo BAR baz");
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::WordForward,
        });
        assert_eq!(editor.text_lines()[0], "FOO BAR baz");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn toggle_word_clears_selection() {
        let mut editor = editor_with("foo BAR baz");
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::WordForward,
        });
        assert_eq!(editor.selection_range(), None);
    }

    #[test]
    fn toggle_lines_clears_selection() {
        let mut editor = editor_with("foo BAR\nbaz QUX");
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::Lines { upward: false },
        });
        assert_eq!(editor.selection_range(), None);
    }

    #[test]
    fn lower_word_forward() {
        let mut editor = editor_with("FOO bar");
        editor.apply(Action::LowercaseAt {
            motion: CaseMotion::WordForward,
        });
        assert_eq!(editor.text_lines()[0], "foo bar");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn upper_word_forward() {
        let mut editor = editor_with("foo BAR baz");
        editor.apply(Action::UppercaseAt {
            motion: CaseMotion::WordForward,
        });
        assert_eq!(editor.text_lines()[0], "FOO BAR baz");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn toggle_word_end() {
        let mut editor = editor_with("foo BAR baz");
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::WordEnd,
        });
        assert_eq!(editor.text_lines()[0], "FOO BAR baz");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn toggle_word_backward() {
        let mut editor = editor_with("foo BAR baz");
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::WordBackward,
        });
        assert_eq!(editor.text_lines()[0], "foo baR baz");
        assert_eq!((editor.row(), editor.col()), (0, 4));
    }

    #[test]
    fn toggle_line_end() {
        let mut editor = editor_with("foo BAR baz");
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::LineEnd,
        });
        assert_eq!(editor.text_lines()[0], "FOO bar BAZ");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn toggle_line_start() {
        let mut editor = editor_with("foo BAR baz");
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::LineStart,
        });
        assert_eq!(editor.text_lines()[0], "FOO BAR baz");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn toggle_line_start_at_zero_is_noop() {
        let mut editor = editor_with("foo");
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::LineStart,
        });
        assert_eq!(editor.text_lines()[0], "foo");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn toggle_lines_down() {
        let mut editor = editor_with("foo BAR\nbaz QUX");
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::Lines { upward: false },
        });
        assert_eq!(editor.text_lines()[0], "FOO bar");
        assert_eq!(editor.text_lines()[1], "BAZ qux");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn toggle_lines_up() {
        let mut editor = editor_with("foo BAR\nbaz QUX");
        editor.apply(Action::MoveDown);
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::Lines { upward: true },
        });
        assert_eq!(editor.text_lines()[0], "FOO bar");
        assert_eq!(editor.text_lines()[1], "BAZ qux");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn toggle_lines_upward_at_top_is_noop() {
        let mut editor = editor_with("a\nb");
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::Lines { upward: true },
        });
        assert_eq!(editor.text_lines(), vec!["a".to_string(), "b".to_string()]);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn toggle_lines_downward_at_bottom_is_noop() {
        let mut editor = editor_with("a\nb");
        editor.apply(Action::MoveDown);
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::Lines { upward: false },
        });
        assert_eq!(editor.text_lines(), vec!["a".to_string(), "b".to_string()]);
        assert_eq!((editor.row(), editor.col()), (1, 0));
    }

    #[test]
    fn toggle_lines_upward_clamps() {
        let mut editor = editor_with("a\nb\nc");
        editor.apply(Action::MoveDown);
        push_digits(&mut editor, &[2]);
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::Lines { upward: true },
        });
        assert_eq!(
            editor.text_lines(),
            vec!["A".to_string(), "B".to_string(), "c".to_string()]
        );
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn toggle_lines_downward_clamps() {
        let mut editor = editor_with("a\nb\nc");
        editor.apply(Action::MoveDown);
        push_digits(&mut editor, &[2]);
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::Lines { upward: false },
        });
        assert_eq!(
            editor.text_lines(),
            vec!["a".to_string(), "B".to_string(), "C".to_string()]
        );
        assert_eq!((editor.row(), editor.col()), (1, 0));
    }

    #[test]
    fn toggle_lines_first_non_blank() {
        let mut editor = editor_with("  aBc\ndef");
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::Lines { upward: false },
        });
        assert_eq!(editor.text_lines()[0], "  AbC");
        assert_eq!((editor.row(), editor.col()), (0, 2));
    }

    #[test]
    fn toggle_lines_skips_empty_row() {
        let mut editor = editor_with("\nb");
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::Lines { upward: false },
        });
        assert_eq!(editor.text_lines()[0], "");
        assert_eq!(editor.text_lines()[1], "B");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn toggle_find_forward() {
        let mut editor = editor_with("axbxc");
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::Find {
                target: 'x',
                forward: true,
                till: false,
            },
        });
        assert_eq!(editor.text_lines()[0], "AXbxc");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn toggle_find_till() {
        let mut editor = editor_with("axbxc");
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::Find {
                target: 'x',
                forward: true,
                till: true,
            },
        });
        assert_eq!(editor.text_lines()[0], "Axbxc");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn toggle_find_backward() {
        let mut editor = editor_with("axbxc");
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::Find {
                target: 'x',
                forward: false,
                till: false,
            },
        });
        assert_eq!(editor.text_lines()[0], "axbXc");
        assert_eq!((editor.row(), editor.col()), (0, 3));
    }

    #[test]
    fn toggle_find_till_backward_is_noop() {
        let mut editor = editor_with("axbxc");
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::Find {
                target: 'x',
                forward: false,
                till: true,
            },
        });
        assert_eq!(editor.text_lines()[0], "axbxc");
        assert_eq!((editor.row(), editor.col()), (0, 4));
    }

    #[test]
    fn toggle_repeat_find() {
        let mut editor = editor_with("axbxc");
        editor.apply(Action::MoveRight);
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::RepeatFind {
                target: 'x',
                forward: true,
                till: false,
            },
        });
        assert_eq!(editor.text_lines()[0], "aXBXc");
        assert_eq!((editor.row(), editor.col()), (0, 1));
    }

    #[test]
    fn toggle_paragraph_single_line() {
        let mut editor = editor_with("foo BAR baz");
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::Object {
                inner: true,
                object: Object::Paragraph,
            },
        });
        assert_eq!(editor.text_lines()[0], "FOO bar BAZ");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn toggle_paren_object() {
        let mut editor = editor_with("foo (BAR) baz");
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::Object {
                inner: true,
                object: Object::Paren,
            },
        });
        assert_eq!(editor.text_lines()[0], "foo (bar) baz");
        assert_eq!((editor.row(), editor.col()), (0, 5));
    }

    #[test]
    fn toggle_paragraph_object() {
        let mut editor = editor_with("a\n\nb");
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::Object {
                inner: true,
                object: Object::Paragraph,
            },
        });
        assert_eq!(editor.text_lines()[0], "A");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn toggle_word_count() {
        let mut editor = editor_with("foo BAR baz qux");
        push_digits(&mut editor, &[2]);
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::WordForward,
        });
        assert_eq!(editor.text_lines()[0], "FOO bar baz qux");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn toggle_leaves_register() {
        let mut editor = editor_with("foo BAR");
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::WordForward,
        });
        assert_eq!(editor.register_text(), String::new());
    }

    #[test]
    fn toggle_goto_last_line() {
        let mut editor = editor_with("foo BAR\nbaz");
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::GotoLine { last: true },
        });
        assert_eq!(editor.text_lines()[0], "FOO bar");
        assert_eq!(editor.text_lines()[1], "BAZ");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn toggle_goto_first_line() {
        let mut editor = editor_with("a\nbb\ncc");
        editor.apply(Action::MoveDown);
        editor.apply(Action::MoveDown);
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::GotoLine { last: false },
        });
        assert_eq!(editor.text_lines()[0], "A");
        assert_eq!(editor.text_lines()[1], "BB");
        assert_eq!(editor.text_lines()[2], "CC");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn toggle_undoes_together() {
        let mut editor = editor_with("foo BAR\nbaz QUX");
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::Lines { upward: false },
        });
        editor.apply(Action::Undo);
        assert_eq!(editor.text_lines()[0], "foo BAR");
        assert_eq!(editor.text_lines()[1], "baz QUX");
    }
}
