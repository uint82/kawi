use super::Editor;
use crate::mode::Mode;
use crate::selection::Selection;

impl Editor {
    // TODO(counts): repeat typed text for insert entry with count like vim 3o.
    pub(super) fn append(&mut self) {
        self.cursor.move_right();
        self.enter_insert();
    }

    pub(super) fn append_line_end(&mut self) {
        let end = self.line_width(self.cursor.row());
        self.cursor.set_position(self.cursor.row(), end);
        self.enter_insert();
    }

    pub(super) fn cancel_command(&mut self) {
        self.mode = Mode::Normal;
        self.command.clear();
    }

    pub(super) fn collapse_selection(&mut self) {
        if self.selection.is_some() {
            self.collapse_to(self.cursor.row(), self.cursor.col());
        }
    }

    pub(super) fn delete_command_char(&mut self) {
        if self.mode == Mode::Command {
            self.command.pop();
        }
    }

    pub(super) fn enter_command(&mut self) {
        self.mode = Mode::Command;
        self.command.clear();
    }

    pub(super) fn enter_insert(&mut self) {
        self.buffer.begin_change();
        self.switch_to_insert();
    }

    pub(super) fn enter_visual(&mut self) {
        self.selection = Some(Selection::new(self.cursor.row(), self.cursor.col()));
        self.mode = Mode::Visual;
    }

    pub(super) fn exit_insert(&mut self) {
        if self.mode == Mode::Insert {
            self.buffer.end_change();
            self.retreat_after_insert();
        }
        self.mode = Mode::Normal;
    }

    pub(super) fn exit_visual(&mut self) {
        self.selection = None;
        self.mode = Mode::Normal;
    }

    pub(super) fn insert_after_selection(&mut self) {
        if let Some((_, (row, col))) = self.selection_range() {
            self.cursor.set_position(row, col);
            self.cursor.move_right();
        }
        self.exit_visual();
        self.enter_insert();
    }

    pub(super) fn insert_before_selection(&mut self) {
        if let Some(((row, col), _)) = self.selection_range() {
            self.cursor.set_position(row, col);
        }
        self.exit_visual();
        self.enter_insert();
    }

    pub(super) fn insert_line_start(&mut self) {
        let row = self.cursor.row();
        let start = self.buffer.insert_start_col(row);
        self.cursor.set_position(row, start);
        self.enter_insert();
    }

    pub(super) fn noop(&mut self) {}

    pub(super) fn normalize_cursor(&mut self) {
        self.clamp_row();
        self.clamp_col();
    }

    pub(super) fn push_command_char(&mut self, c: char) {
        if self.mode == Mode::Command {
            self.command.push(c);
        }
    }

    pub(super) fn redo(&mut self) {
        let count = self.take_count();
        for _ in 0..count {
            if !self.try_redo() {
                break;
            }
        }
    }

    fn retreat_after_insert(&mut self) {
        if self.cursor.col() > 0 {
            self.cursor.move_left();
        }
    }

    pub(super) fn save_silent(&mut self) {
        let _ = self.buffer.save();
    }

    pub(super) fn switch_to_insert(&mut self) {
        self.mode = Mode::Insert;
    }

    pub(super) fn sync_head(&mut self) {
        if let Some(selection) = self.selection.as_mut() {
            selection.set_head(self.cursor.row(), self.cursor.col());
        }
    }

    pub fn try_redo(&mut self) -> bool {
        self.buffer
            .redo_keep_cursor(self.cursor.row(), self.cursor.col())
    }

    pub fn try_undo(&mut self) -> bool {
        match self.buffer.undo(self.cursor.row(), self.cursor.col()) {
            Some((row, col)) => {
                self.cursor.set_position(row, col);
                true
            }
            None => false,
        }
    }

    pub(super) fn undo(&mut self) {
        let count = self.take_count();
        for _ in 0..count {
            if !self.try_undo() {
                break;
            }
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

    fn push_digits(e: &mut Editor, digits: &[u8]) {
        for digit in digits {
            e.apply(Action::PushCount(*digit));
        }
    }

    #[test]
    fn append_at_end_reaches_past_last() {
        let mut e = editor_with("ab");
        e.apply(Action::MoveToLineEnd);
        e.apply(Action::ExitInsert);
        e.apply(Action::Append);
        assert_eq!((e.row(), e.col()), (0, 2));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn append_line_end_groups_typing() {
        let mut e = editor_with("ab");
        e.apply(Action::AppendLineEnd);
        e.apply(Action::InsertChar('c'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "ab");
    }

    #[test]
    fn append_line_end_reaches_end() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::AppendLineEnd);
        assert_eq!((e.row(), e.col()), (0, 6));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn append_on_empty_stays_zero() {
        let mut e = editor_with("");
        e.apply(Action::Append);
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn append_steps_right_and_inserts() {
        let mut e = editor_with("ab");
        e.apply(Action::Append);
        assert_eq!(e.mode(), Mode::Insert);
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn append_then_esc_returns_to_origin() {
        let mut e = editor_with("Sbcdef");
        e.apply(Action::Append);
        assert_eq!((e.row(), e.col()), (0, 1));
        e.apply(Action::ExitInsert);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn collapse_selection_keeps_point() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::CollapseSelection);
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 3), (0, 3))));
    }

    #[test]
    fn command_backspace_shrinks() {
        let mut e = editor_with("ab");
        e.apply(Action::EnterCommand);
        e.apply(Action::PushCommandChar('w'));
        e.apply(Action::DeleteCommandChar);
        assert_eq!(e.command_prompt(), Some(""));
    }

    #[test]
    fn command_buffer_collects_text() {
        let mut e = editor_with("ab");
        e.apply(Action::EnterCommand);
        assert_eq!(e.mode(), Mode::Command);
        e.apply(Action::PushCommandChar('w'));
        e.apply(Action::PushCommandChar('q'));
        assert_eq!(e.command_prompt(), Some("wq"));
    }

    #[test]
    fn command_cancel_returns_to_normal() {
        let mut e = editor_with("ab");
        e.apply(Action::EnterCommand);
        e.apply(Action::PushCommandChar('w'));
        e.apply(Action::CancelCommand);
        assert_eq!(e.mode(), Mode::Normal);
        assert_eq!(e.command_prompt(), None);
    }

    #[test]
    fn delete_selection_undoes_together() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteSelection);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "abcdef");
    }

    #[test]
    fn empty_insert_leaves_no_history() {
        let mut e = editor_with("ab");
        e.apply(Action::EnterInsert);
        e.apply(Action::ExitInsert);
        let before = e.text_lines()[0].to_string();
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], before);
    }

    #[test]
    fn enter_and_exit_insert() {
        let mut e = editor_with("hi");
        e.apply(Action::EnterInsert);
        assert_eq!(e.mode(), Mode::Insert);
        e.apply(Action::ExitInsert);
        assert_eq!(e.mode(), Mode::Normal);
    }

    #[test]
    fn exit_insert_at_start_stays() {
        let mut e = editor_with("abc");
        e.apply(Action::EnterInsert);
        e.apply(Action::ExitInsert);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn exit_insert_pulls_back() {
        let mut e = editor_with("abcdef");
        e.apply(Action::EnterInsert);
        e.apply(Action::MoveToLineEnd);
        assert_eq!(e.col(), 6);
        e.apply(Action::ExitInsert);
        assert_eq!(e.col(), 5);
    }

    #[test]
    fn exit_insert_steps_back() {
        let mut e = editor_with("abcdef");
        e.apply(Action::EnterInsert);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::ExitInsert);
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn insert_after_selection_starts_past_end() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::InsertAfterSelection);
        assert_eq!(e.mode(), Mode::Insert);
        assert_eq!((e.row(), e.col()), (0, 4));
        assert_eq!(e.selection_range(), None);
    }

    #[test]
    fn insert_before_selection_starts_at_start() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::InsertBeforeSelection);
        assert_eq!(e.mode(), Mode::Insert);
        assert_eq!((e.row(), e.col()), (0, 1));
        assert_eq!(e.selection_range(), None);
    }

    #[test]
    fn insert_line_start_blank_goes_to_end() {
        let mut e = editor_with("   ");
        e.apply(Action::InsertLineStart);
        assert_eq!((e.row(), e.col()), (0, 3));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn insert_line_start_finds_content() {
        let mut e = editor_with("  abc");
        e.apply(Action::MoveToLineEnd);
        e.apply(Action::InsertLineStart);
        assert_eq!((e.row(), e.col()), (0, 2));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn insert_session_undoes_together() {
        let mut e = editor_with("ab");
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::InsertChar('Y'));
        e.apply(Action::InsertChar('Z'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "ab");
    }

    #[test]
    fn new_edit_clears_redo() {
        let mut e = editor_with("ab");
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        e.apply(Action::DeleteForward);
        e.apply(Action::Redo);
        assert_eq!(e.text_lines()[0], "b");
    }

    #[test]
    fn open_line_session_undoes_together() {
        let mut e = editor_with("ab");
        e.apply(Action::OpenLineBelow);
        e.apply(Action::InsertChar('H'));
        e.apply(Action::InsertChar('i'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines().len(), 1);
        assert_eq!(e.text_lines()[0], "ab");
    }

    #[test]
    fn redo_keeps_cursor() {
        let mut e = editor_with("aa\nbb");
        e.apply(Action::MoveDown);
        e.apply(Action::OpenLineBelow);
        e.apply(Action::InsertChar('H'));
        e.apply(Action::InsertChar('i'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        assert_eq!((e.row(), e.col()), (1, 0));
        e.apply(Action::Redo);
        assert_eq!(e.text_lines()[2], "Hi");
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn redo_restores_undone_session() {
        let mut e = editor_with("ab");
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        e.apply(Action::Redo);
        assert_eq!(e.text_lines()[0], "Xab");
    }

    #[test]
    fn undo_delete_returns_cursor() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteForward);
        assert_eq!(e.text_lines()[0], "abdef");
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "abcdef");
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn undo_multiline_delete_returns_to_start() {
        let mut e = editor_with("ab\ncd");
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteSelection);
        assert_eq!(e.text_lines().len(), 1);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn undo_open_line_returns_cursor() {
        let mut e = editor_with("aa\nbb");
        e.apply(Action::MoveDown);
        e.apply(Action::OpenLineBelow);
        e.apply(Action::InsertChar('H'));
        e.apply(Action::InsertChar('i'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn undo_session_returns_to_start() {
        let mut e = editor_with("aa");
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::InsertChar('Y'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "aa");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn undo_with_count_undoes_each_change() {
        let mut e = editor_with("foo bar");
        e.apply(Action::DeleteWord);
        e.apply(Action::DeleteWord);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "bar");
        push_digits(&mut e, &[2]);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "foo bar");
    }

    #[test]
    fn redo_with_count_redoes_each_change() {
        let mut e = editor_with("foo bar");
        e.apply(Action::DeleteWord);
        e.apply(Action::DeleteWord);
        e.apply(Action::Undo);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "foo bar");
        push_digits(&mut e, &[2]);
        e.apply(Action::Redo);
        assert_eq!(e.text_lines()[0], "");
    }

    #[test]
    fn repeated_paste_undoes_together() {
        let mut e = editor_with("ab");
        e.apply(Action::YankLine);
        push_digits(&mut e, &[2]);
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines().len(), 3);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines().len(), 1);
        assert_eq!(e.text_lines()[0], "ab");
    }

    #[test]
    fn visual_anchors_at_entry() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 1), (0, 1))));
    }

    #[test]
    fn visual_delete_cuts_range() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteSelection);
        assert_eq!(e.text_lines()[0], "aef");
        assert_eq!((e.row(), e.col()), (0, 1));
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 1), (0, 1))));
    }

    #[test]
    fn visual_exit_keeps_head_clears() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::ExitVisual);
        assert_eq!(e.mode(), Mode::Normal);
        assert_eq!(e.selection_range(), None);
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn visual_motion_down_extends_rows() {
        let mut e = editor_with("abcdef\nghijkl");
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveDown);
        assert_eq!(e.selection_range(), Some(((0, 0), (1, 0))));
    }

    #[test]
    fn visual_motion_extends_head() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        assert_eq!(e.selection_range(), Some(((0, 1), (0, 3))));
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn visual_yank_keeps_text() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::YankSelection);
        assert_eq!(e.text_lines()[0], "abcdef");
        assert_eq!((e.row(), e.col()), (0, 1));
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 1), (0, 3))));
        assert!(!e.is_dirty());
    }
}
