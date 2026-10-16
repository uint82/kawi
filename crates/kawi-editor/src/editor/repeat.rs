use super::session::InsertEdit;
use super::Editor;
use crate::actions::Action;
use crate::editor::case_ops::{lower_ascii_case, toggle_ascii_case, upper_ascii_case};
use crate::mode::Mode;

pub(super) fn is_insert_entry(action: Action) -> bool {
    matches!(
        action,
        Action::EnterInsert
            | Action::Append
            | Action::AppendLineEnd
            | Action::InsertLineStart
            | Action::OpenLineBelow
            | Action::InsertAtLast
            | Action::InsertBeforeSelection
            | Action::InsertAfterSelection
            | Action::ChangeLine
            | Action::ChangeToEnd
            | Action::ChangeToStart
            | Action::ChangeToFirstNonBlank
            | Action::ChangeWord
            | Action::ChangeWordEnd
            | Action::ChangeObject { .. }
            | Action::ChangeInnerWord
            | Action::ChangeAroundWord
            | Action::ChangeFind { .. }
            | Action::ChangeRepeatFind { .. }
            | Action::ChangeSelection
            | Action::ChangeLines
    )
}

#[derive(Clone)]
pub(super) enum RepeatChange {
    Edit {
        action: Action,
        count: usize,
        record: Vec<InsertEdit>,
    },
    Visual {
        action: Action,
        anchor: (usize, usize),
        head: (usize, usize),
        record: Vec<InsertEdit>,
    },
    Undo,
}

impl Editor {
    pub(super) fn repeat_last_change(&mut self) {
        if self.mode.is_visual() {
            return;
        }
        let Some(last) = self.last_change.clone() else {
            return;
        };
        match last {
            RepeatChange::Edit {
                action,
                count,
                record,
            } => self.repeat_edit(action, count, &record),
            RepeatChange::Visual {
                action,
                anchor,
                head,
                record,
            } => self.repeat_visual(action, anchor, head, &record),
            RepeatChange::Undo => {
                self.undo();
            }
        }
    }

    fn repeat_edit(&mut self, action: Action, count: usize, record: &[InsertEdit]) {
        let saved_last = self.last_change.clone();
        let saved_undone = self.undone_change.clone();
        let count = if self.has_count() {
            self.take_count()
        } else {
            count
        };
        self.count_digits = count;
        self.operator_count = None;
        self.apply(action);
        let armed = self.insert_repeat.take();
        for edit in record {
            self.replay_edit(*edit);
        }
        self.insert_repeat = armed;
        if self.mode.is_insert() {
            self.exit_insert();
        }
        self.last_change = saved_last;
        self.undone_change = saved_undone;
    }

    fn repeat_visual(
        &mut self,
        action: Action,
        anchor: (usize, usize),
        head: (usize, usize),
        record: &[InsertEdit],
    ) {
        let (start, end) = self.translated_range(anchor, head);
        match action {
            Action::DeleteSelection | Action::PasteOverSelection => {
                self.record_boundary();
                self.yank_range(start.0, start.1, end.0, end.1);
                self.buffer.delete_range(start.0, start.1, end.0, end.1);
                self.cursor.set_position(start.0, start.1.saturating_sub(1));
            }
            Action::ChangeSelection | Action::ChangeLines => {
                self.record_boundary();
                self.buffer.delete_range(start.0, start.1, end.0, end.1);
                self.cursor.set_position(start.0, start.1);
                self.switch_to_insert();
                for edit in record {
                    self.replay_edit(*edit);
                }
                self.retreat_after_insert();
                self.mode = Mode::Normal;
            }
            Action::ToggleCase | Action::Lowercase | Action::Uppercase => {
                self.record_boundary();
                let shift = match action {
                    Action::Lowercase => lower_ascii_case,
                    Action::Uppercase => upper_ascii_case,
                    _ => toggle_ascii_case,
                };
                self.buffer
                    .shift_case_range(start.0, start.1, end.0, end.1, shift);
                self.cursor.set_position(start.0, start.1);
            }
            _ => {}
        }
    }

    fn translated_range(
        &self,
        anchor: (usize, usize),
        head: (usize, usize),
    ) -> ((usize, usize), (usize, usize)) {
        let (saved_start, saved_end) = if anchor <= head {
            (anchor, head)
        } else {
            (head, anchor)
        };
        let cursor = (self.cursor.row(), self.cursor.col());
        let end = (
            cursor.0 + saved_end.0.saturating_sub(saved_start.0),
            cursor.1 + saved_end.1.saturating_sub(saved_start.1),
        );
        (cursor, end)
    }

    pub(super) fn record_change(&mut self, change: RepeatChange) {
        self.last_change = Some(change);
        self.undone_change = None;
    }

    pub(super) fn note_visual_repeat(&mut self, action: Action) {
        if !self.mode.is_visual() {
            return;
        }
        let Some(selection) = self.selection else {
            return;
        };
        self.record_change(RepeatChange::Visual {
            action,
            anchor: selection.anchor(),
            head: selection.head(),
            record: Vec::new(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{Action, CaseMotion};
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
    fn repeat_delete_line_after_move() {
        let mut editor = editor_with("a\nb\nc");
        editor.apply(Action::DeleteLine);
        editor.apply(Action::MoveDown);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(editor.text_lines()[0], "b");
        assert_eq!(editor.line_count(), 1);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn repeat_counted_delete_past_end_is_noop() {
        let mut editor = editor_with("a\nb\nc\nd");
        push_digits(&mut editor, &[2]);
        editor.apply(Action::DeleteLine);
        editor.apply(Action::MoveDown);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(editor.text_lines()[0], "c");
        assert_eq!(editor.text_lines()[1], "d");
        assert_eq!((editor.row(), editor.col()), (1, 0));
    }

    #[test]
    fn repeat_fresh_count_replaces() {
        let mut editor = editor_with("a\nb\nc\nd");
        editor.apply(Action::DeleteLine);
        push_digits(&mut editor, &[2]);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(editor.text_lines()[0], "d");
        assert_eq!(editor.line_count(), 1);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn repeat_counted_with_fresh_count() {
        let mut editor = editor_with("a\nb\nc\nd\ne\nf\ng");
        push_digits(&mut editor, &[2]);
        editor.apply(Action::DeleteLine);
        push_digits(&mut editor, &[3]);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(editor.text_lines()[0], "f");
        assert_eq!(editor.text_lines()[1], "g");
    }

    #[test]
    fn repeat_delete_word() {
        let mut editor = editor_with("foo bar\nbaz");
        editor.apply(Action::DeleteWord);
        editor.apply(Action::MoveDown);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(editor.text_lines()[0], "bar");
        assert_eq!(editor.text_lines()[1], "");
        assert_eq!((editor.row(), editor.col()), (1, 0));
    }

    #[test]
    fn repeat_delete_to_end() {
        let mut editor = editor_with("abcdef\ngh");
        for _ in 0..3 {
            editor.apply(Action::MoveRight);
        }
        editor.apply(Action::DeleteToEnd);
        editor.apply(Action::MoveDown);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(editor.text_lines()[0], "abc");
        assert_eq!(editor.text_lines()[1], "g");
        assert_eq!((editor.row(), editor.col()), (1, 0));
    }

    #[test]
    fn repeat_delete_forward() {
        let mut editor = editor_with("abcdef");
        editor.apply(Action::MoveRight);
        editor.apply(Action::DeleteForward);
        editor.apply(Action::MoveRight);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(editor.text_lines()[0], "acef");
        assert_eq!((editor.row(), editor.col()), (0, 2));
    }

    #[test]
    fn repeat_change_line_text() {
        let mut editor = editor_with("aaa\nbbb");
        editor.apply(Action::ChangeLine);
        editor.apply(Action::InsertChar('X'));
        editor.apply(Action::ExitInsert);
        editor.apply(Action::MoveDown);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(editor.text_lines()[0], "X");
        assert_eq!(editor.text_lines()[1], "X");
        assert_eq!((editor.row(), editor.col()), (1, 0));
    }

    #[test]
    fn repeat_change_count() {
        let mut editor = editor_with("a\nb\nc\nd");
        push_digits(&mut editor, &[2]);
        editor.apply(Action::ChangeLine);
        editor.apply(Action::InsertChar('X'));
        editor.apply(Action::ExitInsert);
        editor.apply(Action::MoveDown);
        push_digits(&mut editor, &[2]);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(editor.text_lines()[0], "X");
        assert_eq!(editor.text_lines()[1], "X");
        assert_eq!(editor.line_count(), 2);
        assert_eq!((editor.row(), editor.col()), (1, 0));
    }

    #[test]
    fn repeat_shift_line() {
        let mut editor = editor_with("a\nb");
        editor.apply(Action::ShiftRight);
        editor.apply(Action::MoveDown);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(editor.text_lines()[0], "\ta");
        assert_eq!(editor.text_lines()[1], "\tb");
    }

    #[test]
    fn repeat_toggle_word() {
        let mut editor = editor_with("foo BAR\nqux BAZ");
        editor.apply(Action::ToggleCaseAt {
            motion: CaseMotion::WordForward,
        });
        editor.apply(Action::MoveDown);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(editor.text_lines()[0], "FOO BAR");
        assert_eq!(editor.text_lines()[1], "QUX BAZ");
    }

    #[test]
    fn repeat_put() {
        let mut editor = editor_with("abcdef\nghijkl");
        editor.apply(Action::MoveRight);
        editor.apply(Action::YankToEnd);
        editor.apply(Action::MoveDown);
        editor.apply(Action::PasteAfter);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(editor.text_lines()[1], "ghbcdefbcdefijkl");
        assert_eq!((editor.row(), editor.col()), (1, 11));
    }

    #[test]
    fn repeat_yank_does_not_clobber() {
        let mut editor = editor_with("a\nb\nc");
        editor.apply(Action::DeleteLine);
        editor.apply(Action::YankLine);
        editor.apply(Action::MoveDown);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(editor.text_lines()[0], "b");
        assert_eq!(editor.line_count(), 1);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn repeat_undo_redoes_undo() {
        let mut editor = editor_with("a\nb");
        editor.apply(Action::DeleteLine);
        editor.apply(Action::Undo);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(editor.text_lines(), vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn repeat_redo_restores_change() {
        let mut editor = editor_with("a\nb");
        editor.apply(Action::DeleteLine);
        editor.apply(Action::Undo);
        editor.apply(Action::Redo);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(editor.text_lines(), vec![String::new()]);
    }

    #[test]
    fn repeat_open_line_inserts() {
        let mut editor = editor_with("a\nb");
        editor.apply(Action::OpenLineBelow);
        editor.apply(Action::InsertChar('X'));
        editor.apply(Action::ExitInsert);
        editor.apply(Action::MoveDown);
        editor.apply(Action::MoveDown);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(editor.text_lines()[2], "b");
        assert_eq!(editor.text_lines()[3], "X");
    }

    #[test]
    fn repeat_visual_delete_shifts_range() {
        let mut editor = editor_with("abcdef");
        editor.apply(Action::MoveRight);
        editor.apply(Action::EnterVisual);
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::DeleteSelection);
        editor.apply(Action::ExitVisual);
        editor.apply(Action::MoveRight);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(editor.text_lines()[0], "ae");
        assert_eq!((editor.row(), editor.col()), (0, 1));
    }

    #[test]
    fn repeat_visual_change_inserts_text() {
        let mut editor = editor_with("abcdef");
        editor.apply(Action::MoveRight);
        editor.apply(Action::EnterVisual);
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::ChangeSelection);
        editor.apply(Action::InsertChar('X'));
        editor.apply(Action::ExitInsert);
        editor.apply(Action::MoveRight);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(editor.text_lines()[0], "aXX");
        assert_eq!((editor.row(), editor.col()), (0, 2));
    }

    #[test]
    fn repeat_visual_toggle_shifts_range() {
        let mut editor = editor_with("abcdef");
        editor.apply(Action::MoveRight);
        editor.apply(Action::EnterVisual);
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::ToggleCase);
        editor.apply(Action::ExitVisual);
        editor.apply(Action::MoveRight);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(editor.text_lines()[0], "aBcdEf");
        assert_eq!((editor.row(), editor.col()), (0, 2));
    }

    #[test]
    fn repeat_counted_open_line() {
        let mut editor = editor_with("a\nb");
        push_digits(&mut editor, &[2]);
        editor.apply(Action::OpenLineBelow);
        editor.apply(Action::InsertChar('X'));
        editor.apply(Action::ExitInsert);
        editor.apply(Action::MoveDown);
        editor.apply(Action::MoveDown);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(editor.text_lines()[3], "b");
        assert_eq!(editor.text_lines()[4], "X");
        assert_eq!(editor.text_lines()[5], "X");
        assert_eq!(editor.line_count(), 6);
        assert_eq!((editor.row(), editor.col()), (5, 0));
    }

    #[test]
    fn repeat_visual_line_delete_clamps() {
        let mut editor = editor_with("a\nb\nc");
        editor.apply(Action::EnterVisualLine);
        editor.apply(Action::MoveDown);
        editor.apply(Action::DeleteSelection);
        editor.apply(Action::ExitVisual);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(editor.text_lines(), vec![String::new()]);
    }

    #[test]
    fn repeat_fresh_is_noop() {
        let mut editor = editor_with("ab");
        editor.apply(Action::MoveRight);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(editor.text_lines()[0], "ab");
        assert_eq!((editor.row(), editor.col()), (0, 1));
    }

    #[test]
    fn repeat_ignores_visual_mode() {
        let mut editor = editor_with("abcdef");
        editor.apply(Action::DeleteLine);
        editor.apply(Action::Undo);
        editor.apply(Action::EnterVisual);
        editor.apply(Action::RepeatLastChange);
        assert_eq!(editor.mode(), crate::mode::Mode::Visual);
        assert_eq!(editor.text_lines()[0], "abcdef");
    }
}
