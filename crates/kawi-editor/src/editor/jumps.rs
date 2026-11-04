use super::{EditError, Editor};
use crate::jumps::Jump;

impl Editor {
    pub(super) fn set_pcmark(&mut self) {
        let position = (self.cursor.row(), self.cursor.col());
        self.set_pcmark_at(position);
    }

    pub(super) fn set_pcmark_at(&mut self, (row, col): (usize, usize)) {
        self.jumps.record(Jump {
            buffer: self.buffers.current(),
            row,
            col,
        });
    }

    pub(super) fn apply_line_adjustments(&mut self) {
        let buffer = self.buffers.current();
        let path = self.absolute_path();
        for adjust in self.buffer.take_line_adjustments() {
            self.jumps.adjust(buffer, adjust);
            self.marks.adjust(buffer, path.as_deref(), adjust);
            self.folds.adjust(adjust);
        }
    }

    pub fn record_jump(&mut self) {
        self.set_pcmark();
    }

    pub fn jump_older(&mut self) -> Result<(), EditError> {
        self.jump_step(true)
    }

    pub fn jump_newer(&mut self) -> Result<(), EditError> {
        self.jump_step(false)
    }

    fn jump_step(&mut self, older: bool) -> Result<(), EditError> {
        let count = self.take_count();
        let cursor = self.current_jump();
        let found = if older {
            self.jumps.older(count, cursor)
        } else {
            self.jumps.newer(count, cursor)
        };
        let Some(target) = found else {
            return Ok(());
        };
        if !self.jump_target_valid(target) {
            return Err(EditError::InvalidMark);
        }
        self.go_to_jump(target);
        Ok(())
    }

    fn jump_target_valid(&self, target: Jump) -> bool {
        let lines = if target.buffer == self.buffers.current() {
            self.buffer.line_count()
        } else {
            match self.buffers.line_count(target.buffer) {
                Some(lines) => lines,
                None => return false,
            }
        };
        target.row < lines
    }

    fn go_to_jump(&mut self, target: Jump) {
        if target.buffer != self.buffers.current() {
            self.switch_buffer(target.buffer);
        }
        let col = target.col.min(self.max_col_for_row(target.row));
        self.cursor.set_position(target.row, col);
        self.remember_wanted_column();
        self.reveal_cursor();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use crate::search::SearchOutcome;
    use kawi_buffer::Buffer;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    fn positions(entries: &[Jump]) -> Vec<(usize, usize)> {
        entries.iter().map(|entry| (entry.row, entry.col)).collect()
    }

    fn search(editor: &mut Editor, pattern: &str) -> Option<SearchOutcome> {
        editor.apply(Action::EnterSearch);
        for c in pattern.chars() {
            editor.apply(Action::PushSearchChar(c));
        }
        editor.confirm_search()
    }

    #[test]
    fn jump_older_returns_to_previous_position() {
        let mut editor = editor_with("ab\ncd\nef");
        editor.apply(Action::GotoLastLine);
        editor.apply(Action::JumpOlder);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn jump_newer_returns_forward() {
        let mut editor = editor_with("ab\ncd\nef");
        editor.apply(Action::GotoLastLine);
        editor.apply(Action::JumpOlder);
        editor.apply(Action::JumpNewer);
        assert_eq!((editor.row(), editor.col()), (2, 0));
    }

    #[test]
    fn counted_jump_skips_entries() {
        let mut editor = editor_with("ab\ncd\nef");
        editor.apply(Action::PushCount(3));
        editor.apply(Action::GotoLastLine);
        editor.apply(Action::PushCount(2));
        editor.apply(Action::GotoLastLine);
        editor.apply(Action::PushCount(2));
        editor.apply(Action::JumpOlder);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn successful_search_records_origin() {
        let mut editor = editor_with("line 1\nline 2\nline 3");
        editor.apply(Action::JumpTo(2, 0));
        assert_eq!(
            search(&mut editor, "line 1"),
            Some(SearchOutcome::Found { wrapped: true })
        );
        editor.apply(Action::MoveDown);
        editor.apply(Action::JumpOlder);
        assert_eq!((editor.row(), editor.col()), (2, 0));
    }

    #[test]
    fn failed_search_does_not_record() {
        let mut editor = editor_with("line 1\nline 2");
        editor.apply(Action::MoveDown);
        assert_eq!(search(&mut editor, "zzz"), Some(SearchOutcome::Missing));
        editor.apply(Action::JumpOlder);
        assert_eq!((editor.row(), editor.col()), (1, 0));
    }

    #[test]
    fn failed_search_restores_origin_after_partial_match() {
        let mut editor = editor_with("line 1\nline 2");
        editor.apply(Action::MoveDown);
        assert_eq!(search(&mut editor, "line 1x"), Some(SearchOutcome::Missing));
        assert_eq!((editor.row(), editor.col()), (1, 0));
    }

    #[test]
    fn failed_next_match_does_not_record() {
        let mut editor = editor_with("line 1\nline 2");
        editor.apply(Action::NextMatch);
        editor.apply(Action::MoveDown);
        editor.apply(Action::JumpOlder);
        assert_eq!((editor.row(), editor.col()), (1, 0));
    }

    #[test]
    fn percent_match_records() {
        let mut editor = editor_with("f(a)b\ncd");
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveToMatch);
        assert_eq!((editor.row(), editor.col()), (0, 3));
        editor.apply(Action::MoveDown);
        editor.apply(Action::JumpOlder);
        assert_eq!((editor.row(), editor.col()), (0, 1));
    }

    #[test]
    fn percent_mismatch_does_not_record() {
        let mut editor = editor_with("ab\ncd");
        editor.apply(Action::MoveToMatch);
        editor.apply(Action::MoveDown);
        editor.apply(Action::JumpOlder);
        assert_eq!((editor.row(), editor.col()), (1, 0));
    }

    #[test]
    fn counted_percent_records_without_movement() {
        let mut editor = editor_with("ab\ncd\nef");
        editor.apply(Action::PushCount(1));
        editor.apply(Action::MoveToMatch);
        editor.apply(Action::MoveDown);
        editor.apply(Action::JumpOlder);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn viewport_top_records_without_movement() {
        let mut editor = editor_with("ab\ncd");
        editor.apply(Action::MoveToViewportTop);
        editor.apply(Action::MoveDown);
        editor.apply(Action::JumpOlder);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn failed_paragraph_count_does_not_record() {
        let mut editor = editor_with("ab\ncd\nef");
        editor.apply(Action::PushCount(3));
        editor.apply(Action::MoveToNextParagraph);
        editor.apply(Action::MoveDown);
        editor.apply(Action::JumpOlder);
        assert_eq!((editor.row(), editor.col()), (1, 0));
    }

    #[test]
    fn duplicate_rows_dedupe_on_navigation() {
        let mut editor = editor_with("ab\ncd");
        editor.apply(Action::GotoLastLine);
        editor.apply(Action::GotoLastLine);
        assert_eq!(positions(editor.jumps.entries()), vec![(0, 0), (1, 0)]);
        editor.apply(Action::JumpOlder);
        assert_eq!(positions(editor.jumps.entries()), vec![(0, 0), (1, 0)]);
        assert_eq!(editor.jumps.index(), 0);
    }

    #[test]
    fn delete_line_adjusts_recorded_jumps() {
        let mut editor = editor_with("a\nb\nc\nd");
        editor.apply(Action::GotoLastLine);
        editor.apply(Action::PushCount(3));
        editor.apply(Action::GotoLastLine);
        editor.apply(Action::DeleteLine);
        assert_eq!(positions(editor.jumps.entries()), vec![(0, 0), (2, 0)]);
    }

    #[test]
    fn open_line_adjusts_recorded_jumps() {
        let mut editor = editor_with("a\nb\nc\nd");
        editor.apply(Action::GotoLastLine);
        editor.apply(Action::PushCount(3));
        editor.apply(Action::GotoLastLine);
        editor.apply(Action::OpenLineBelow);
        editor.apply(Action::ExitInsert);
        assert_eq!(positions(editor.jumps.entries()), vec![(0, 0), (4, 0)]);
    }

    #[test]
    fn paste_below_adjusts_recorded_jumps() {
        let mut editor = editor_with("a\nb\nc\nd");
        editor.apply(Action::GotoLastLine);
        editor.apply(Action::PushCount(3));
        editor.apply(Action::GotoLastLine);
        editor.apply(Action::YankLine);
        editor.apply(Action::PasteAfter);
        assert_eq!(positions(editor.jumps.entries()), vec![(0, 0), (4, 0)]);
    }

    #[test]
    fn undo_records_and_adjusts_jumps() {
        let mut editor = editor_with("a\nb\nc\nd");
        editor.apply(Action::GotoLastLine);
        editor.apply(Action::PushCount(3));
        editor.apply(Action::GotoLastLine);
        editor.apply(Action::DeleteLine);
        editor.apply(Action::Undo);
        assert_eq!(
            positions(editor.jumps.entries()),
            vec![(0, 0), (3, 0), (3, 0)]
        );
        assert_eq!(editor.jumps.index(), 3);
    }

    #[test]
    fn redo_records_and_adjusts_jumps() {
        let mut editor = editor_with("a\nb\nc\nd");
        editor.apply(Action::GotoLastLine);
        editor.apply(Action::PushCount(3));
        editor.apply(Action::GotoLastLine);
        editor.apply(Action::DeleteLine);
        editor.apply(Action::Undo);
        editor.apply(Action::Redo);
        assert_eq!(
            positions(editor.jumps.entries()),
            vec![(0, 0), (2, 0), (2, 0), (2, 0)]
        );
    }

    #[test]
    fn undo_without_history_keeps_jumps() {
        let mut editor = editor_with("ab");
        editor.apply(Action::GotoLastLine);
        editor.apply(Action::Undo);
        assert_eq!(positions(editor.jumps.entries()), vec![(0, 0)]);
        assert_eq!(editor.jumps.index(), 1);
    }

    #[test]
    fn move_down_at_last_line_keeps_column() {
        let mut editor = editor_with("ab\ncd");
        editor.apply(Action::JumpTo(1, 1));
        editor.apply(Action::MoveDown);
        assert_eq!((editor.row(), editor.col()), (1, 1));
    }

    #[test]
    fn move_up_at_first_line_keeps_column() {
        let mut editor = editor_with("ab\ncd");
        editor.apply(Action::JumpTo(0, 1));
        editor.apply(Action::MoveUp);
        assert_eq!((editor.row(), editor.col()), (0, 1));
    }
}
