use super::Editor;

impl Editor {
    pub(super) fn set_pcmark(&mut self) {
        self.set_pcmark_at((self.cursor.row(), self.cursor.col()));
    }

    pub(super) fn set_pcmark_at(&mut self, position: (usize, usize)) {
        self.jumps.record(position);
    }

    pub(super) fn jump_older(&mut self) {
        self.jump_step(true);
    }

    pub(super) fn jump_newer(&mut self) {
        self.jump_step(false);
    }

    fn jump_step(&mut self, older: bool) {
        let count = self.take_count();
        let cursor = (self.cursor.row(), self.cursor.col());
        let found = if older {
            self.jumps.older(count, cursor)
        } else {
            self.jumps.newer(count, cursor)
        };
        if let Some((row, col)) = found {
            self.cursor.set_position(row, col);
            self.reveal_jump();
        }
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
            Some(SearchOutcome::Found { wrapped: false })
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
        assert_eq!(editor.jumps.entries(), &[(0, 0), (1, 0)]);
        editor.apply(Action::JumpOlder);
        assert_eq!(editor.jumps.entries(), &[(0, 0), (1, 0)]);
        assert_eq!(editor.jumps.index(), 0);
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
