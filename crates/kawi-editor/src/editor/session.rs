use super::Editor;
use crate::actions::Object;
use crate::mode::Mode;
use crate::objects;
use crate::selection::{SavedVisual, Selection};
use crate::words;

#[derive(Clone, Copy)]
pub(super) enum InsertEdit {
    Char(char),
    Newline,
    Backspace,
    Delete,
}

impl Editor {
    pub(super) fn append(&mut self) {
        self.arm_insert_repeat(false);
        self.cursor.move_right();
        self.enter_insert();
    }

    pub(super) fn append_line_end(&mut self) {
        self.arm_insert_repeat(false);
        let end = self.line_width(self.cursor.row());
        self.cursor.set_position(self.cursor.row(), end);
        self.enter_insert();
    }

    pub(super) fn arm_insert_repeat(&mut self, reopen: bool) {
        let count = self.take_count();
        self.insert_repeat = if count > 1 {
            Some(count.saturating_sub(1))
        } else {
            None
        };
        self.insert_reopen = reopen;
        self.insert_record.clear();
        self.insert_moved = false;
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

    pub(super) fn insert_at_last(&mut self) {
        if let Some(pos) = self.last_insert {
            let row = pos.0.min(self.buffer.line_count().saturating_sub(1));
            let col = pos.1.min(self.buffer.line_len(row));
            self.cursor.set_position(row, col);
        }
        self.arm_insert_repeat(false);
        self.enter_insert();
    }

    pub(super) fn enter_visual(&mut self) {
        if self.selection.is_none() {
            self.selection = Some(Selection::new(self.cursor.row(), self.cursor.col()));
        }
        self.mode = Mode::Visual;
    }

    pub(super) fn enter_visual_line(&mut self) {
        if self.selection.is_none() {
            self.selection = Some(Selection::new(self.cursor.row(), self.cursor.col()));
        }
        self.mode = Mode::VisualLine;
    }

    pub(super) fn swap_selection_ends(&mut self) {
        if let Some(selection) = self.selection.as_mut() {
            selection.swap_ends();
            let head = selection.head();
            self.cursor.set_position(head.0, head.1);
        }
    }

    pub(super) fn exit_insert(&mut self) {
        if self.mode == Mode::Insert {
            self.last_insert = Some((self.cursor.row(), self.cursor.col()));
            self.replay_insert();
            self.buffer.end_change();
            self.retreat_after_insert();
        }
        self.mode = Mode::Normal;
        self.insert_repeat = None;
        self.insert_reopen = false;
        self.insert_record.clear();
        self.insert_moved = false;
    }

    fn replay_insert(&mut self) {
        let repeats = self.insert_repeat.take().unwrap_or(0);
        if self.insert_moved || repeats == 0 {
            return;
        }
        let record = std::mem::take(&mut self.insert_record);
        let reopen = std::mem::replace(&mut self.insert_reopen, false);
        for _ in 0..repeats {
            if reopen {
                self.replay_open_line();
            }
            for edit in &record {
                self.replay_edit(*edit);
            }
        }
    }

    fn replay_open_line(&mut self) {
        self.open_line_after(self.cursor.row());
    }

    fn replay_edit(&mut self, edit: InsertEdit) {
        match edit {
            InsertEdit::Char(c) => self.insert_char(c),
            InsertEdit::Newline => self.insert_newline(),
            InsertEdit::Backspace => self.delete_backward(),
            InsertEdit::Delete => self.delete_forward(),
        }
    }

    pub(super) fn record_insert(&mut self, edit: InsertEdit) {
        if self.insert_repeat.is_some() {
            self.insert_record.push(edit);
        }
    }

    pub(super) fn track_insert_move(&mut self) {
        if self.mode.is_insert() && self.insert_repeat.is_some() {
            self.insert_moved = true;
        }
    }

    pub(super) fn exit_visual(&mut self) {
        self.selection = None;
        self.mode = Mode::Normal;
    }

    pub(super) fn snapshot_visual(&mut self) {
        if !self.mode.is_visual() {
            return;
        }
        if let Some(selection) = self.selection {
            self.previous_visual = Some(SavedVisual::new(selection, self.mode));
        }
    }

    pub(super) fn cancel_visual(&mut self) {
        self.snapshot_visual();
        self.exit_visual();
    }

    pub(super) fn reselect_visual(&mut self) {
        let Some(saved) = self.previous_visual else {
            return;
        };
        if self.mode.is_visual() {
            let Some(current) = self.selection else {
                return;
            };
            self.previous_visual = Some(SavedVisual::new(current, self.mode));
        }
        self.selection = Some(saved.selection());
        self.mode = saved.mode();
        self.clamp_saved_selection();
    }

    fn clamp_saved_selection(&mut self) {
        let Some(selection) = self.selection else {
            return;
        };
        let anchor = self.clamp_saved_pos(selection.anchor());
        let head = self.clamp_saved_pos(selection.head());
        self.selection = Some(Selection::from_ends(anchor, head));
        self.cursor.set_position(head.0, head.1);
    }

    fn clamp_saved_pos(&self, pos: (usize, usize)) -> (usize, usize) {
        let row = pos.0.min(self.buffer.line_count().saturating_sub(1));
        (row, pos.1.min(self.max_col_for_row(row)))
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
        self.arm_insert_repeat(false);
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

    pub(super) fn select_around_word(&mut self) {
        self.select_word_object(false);
    }

    pub(super) fn select_inner_word(&mut self) {
        self.select_word_object(true);
    }

    pub(super) fn select_object(&mut self, inner: bool, object: Object) {
        let (row, col) = (self.cursor.row(), self.cursor.col());
        let count = self.take_count();
        let span = match object {
            Object::Paren => {
                objects::brackets::bracket_range(&self.buffer, row, col, '(', ')', inner, count)
            }
            Object::Brace => {
                objects::brackets::bracket_range(&self.buffer, row, col, '{', '}', inner, count)
            }
            Object::Bracket => {
                objects::brackets::bracket_range(&self.buffer, row, col, '[', ']', inner, count)
            }
            Object::Angle => {
                objects::brackets::bracket_range(&self.buffer, row, col, '<', '>', inner, count)
            }
            Object::SingleQuote => {
                objects::quotes::quote_range(&self.buffer, row, col, '\'', inner, count)
            }
            Object::DoubleQuote => {
                objects::quotes::quote_range(&self.buffer, row, col, '"', inner, count)
            }
            Object::Backtick => {
                objects::quotes::quote_range(&self.buffer, row, col, '`', inner, count)
            }
            Object::Paragraph => {
                return self.select_paragraph(inner, count);
            }
        };
        let Some(span) = span else {
            return;
        };
        match span {
            words::WordSpan::Charwise { start, end } => {
                self.selection = Some(Selection::from_ends(start, end));
                self.cursor.set_position(end.0, end.1);
            }
            words::WordSpan::Linewise { first, last } => {
                let end = (last, self.line_width(last).saturating_sub(1));
                self.selection = Some(Selection::from_ends((first, 0), end));
                self.cursor.set_position(end.0, end.1);
            }
            words::WordSpan::Empty { .. } => {
                return;
            }
        }
        self.downgrade_to_charwise();
    }

    fn select_paragraph(&mut self, inner: bool, count: usize) {
        let anchor = self.selection_anchor();
        let head = self.selection_head();
        if head.0 != anchor.0 {
            self.extend_paragraph_head(head.0 < anchor.0, !inner, count);
            return;
        }
        let row = self.cursor.row();
        let Some(span) = objects::paragraph::paragraph_range(&self.buffer, row, !inner, count)
        else {
            return;
        };
        let words::WordSpan::Linewise { first, last } = span else {
            return;
        };
        if self.mode == Mode::VisualLine && first == row {
            self.extend_paragraph_head(row < anchor.0, !inner, count);
            return;
        }
        self.select_lines(first, last);
    }

    fn extend_paragraph_head(&mut self, backward: bool, include: bool, count: usize) {
        let mut line = self.selection_head().0;
        for _ in 0..count {
            line = objects::paragraph::paragraph_step(&self.buffer, line, !backward, include);
        }
        if let Some(selection) = self.selection.as_mut() {
            selection.set_head(line, 0);
        }
        self.cursor.set_position(line, 0);
    }

    fn downgrade_to_charwise(&mut self) {
        if self.mode == Mode::VisualLine {
            self.mode = Mode::Visual;
        }
    }

    fn select_word_object(&mut self, inner: bool) {
        let count = self.take_count().max(1);
        let cursor = (self.cursor.row(), self.cursor.col());
        let (anchor, head) = match &self.selection {
            Some(selection) => (selection.anchor(), selection.head()),
            None => {
                self.selection = Some(Selection::new(cursor.0, cursor.1));
                (cursor, cursor)
            }
        };
        if anchor != head {
            let head = words::extend_word_head(&self.buffer, anchor, head, inner, count);
            self.place_selection_head(head);
            self.downgrade_to_charwise();
            return;
        }
        let span = if inner {
            objects::iw_range(&self.buffer, cursor.0, cursor.1)
        } else {
            objects::aw_range(&self.buffer, cursor.0, cursor.1)
        };
        let Some(span) = span else {
            return;
        };
        self.select_span(span.start, span.end);
        self.downgrade_to_charwise();
        let head = self.selection_head();
        let next = words::extend_word_head(
            &self.buffer,
            self.selection_anchor(),
            head,
            inner,
            count.saturating_sub(1),
        );
        self.place_selection_head(next);
    }

    fn select_span(&mut self, start: (usize, usize), end: (usize, usize)) {
        let end = if start == end {
            start
        } else {
            words::prev_char_pos(&self.buffer, end.0, end.1).unwrap_or(start)
        };
        self.selection = Some(Selection::from_ends(start, end));
        self.cursor.set_position(end.0, end.1);
    }

    fn select_lines(&mut self, first: usize, last: usize) {
        self.selection = Some(Selection::from_ends((first, 0), (last, 0)));
        self.cursor.set_position(last, 0);
        self.mode = Mode::VisualLine;
    }

    fn selection_anchor(&self) -> (usize, usize) {
        self.selection
            .as_ref()
            .map_or((self.cursor.row(), self.cursor.col()), |selection| {
                selection.anchor()
            })
    }

    fn selection_head(&self) -> (usize, usize) {
        self.selection
            .as_ref()
            .map_or((self.cursor.row(), self.cursor.col()), |selection| {
                selection.head()
            })
    }

    fn place_selection_head(&mut self, head: (usize, usize)) {
        if let Some(selection) = self.selection.as_mut() {
            selection.set_head(head.0, head.1);
        }
        self.cursor.set_position(head.0, head.1);
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
    fn append_with_count_repeats_typed() {
        let mut e = editor_with("ab");
        push_digits(&mut e, &[2]);
        e.apply(Action::Append);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "aXXb");
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn enter_insert_with_count_repeats_typed() {
        let mut e = editor_with("ab");
        push_digits(&mut e, &[2]);
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "XXab");
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn append_line_end_with_count_repeats_typed() {
        let mut e = editor_with("ab");
        push_digits(&mut e, &[2]);
        e.apply(Action::AppendLineEnd);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "abXX");
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn insert_line_start_with_count_repeats_typed() {
        let mut e = editor_with("  ab");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        push_digits(&mut e, &[2]);
        e.apply(Action::InsertLineStart);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "  XXab");
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn open_line_below_with_count_repeats_per_line() {
        let mut e = editor_with("ab");
        push_digits(&mut e, &[2]);
        e.apply(Action::OpenLineBelow);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines().len(), 3);
        assert_eq!(e.text_lines()[1], "X");
        assert_eq!(e.text_lines()[2], "X");
        assert_eq!((e.row(), e.col()), (2, 0));
    }

    #[test]
    fn open_line_below_with_count_three_repeats_twice() {
        let mut e = editor_with("ab");
        push_digits(&mut e, &[3]);
        e.apply(Action::OpenLineBelow);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines().len(), 4);
        assert_eq!(e.text_lines()[1], "X");
        assert_eq!(e.text_lines()[2], "X");
        assert_eq!(e.text_lines()[3], "X");
        assert_eq!((e.row(), e.col()), (3, 0));
    }

    #[test]
    fn open_line_below_with_count_copies_indent() {
        let mut e = editor_with("  ab");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        push_digits(&mut e, &[2]);
        e.apply(Action::OpenLineBelow);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines().len(), 3);
        assert_eq!(e.text_lines()[1], "  X");
        assert_eq!(e.text_lines()[2], "  X");
        assert_eq!((e.row(), e.col()), (2, 2));
    }

    #[test]
    fn open_line_below_with_count_empty_session() {
        let mut e = editor_with("ab");
        push_digits(&mut e, &[2]);
        e.apply(Action::OpenLineBelow);
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines().len(), 3);
        assert_eq!((e.row(), e.col()), (2, 0));
    }

    #[test]
    fn repeat_undoes_together() {
        let mut e = editor_with("ab");
        push_digits(&mut e, &[2]);
        e.apply(Action::OpenLineBelow);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines().len(), 1);
        assert_eq!(e.text_lines()[0], "ab");
    }

    #[test]
    fn repeat_append_undoes_together() {
        let mut e = editor_with("ab");
        push_digits(&mut e, &[2]);
        e.apply(Action::Append);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "ab");
    }

    #[test]
    fn repeat_with_newline_replays_lines() {
        let mut e = editor_with("ab");
        push_digits(&mut e, &[2]);
        e.apply(Action::OpenLineBelow);
        e.apply(Action::InsertNewline);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines().len(), 5);
        assert_eq!(e.text_lines()[2], "X");
        assert_eq!(e.text_lines()[4], "X");
        assert_eq!((e.row(), e.col()), (4, 0));
    }

    #[test]
    fn repeat_with_backspace_replays_keys() {
        let mut e = editor_with("ab");
        e.apply(Action::MoveRight);
        push_digits(&mut e, &[2]);
        e.apply(Action::Append);
        e.apply(Action::DeleteBackward);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "aX");
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn arrow_cancels_repeat() {
        let mut e = editor_with("ab");
        push_digits(&mut e, &[2]);
        e.apply(Action::Append);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::MoveLeft);
        e.apply(Action::InsertChar('Y'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "aYXb");
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn change_with_count_does_not_repeat() {
        let mut e = editor_with("foo bar baz");
        push_digits(&mut e, &[2]);
        e.apply(Action::BeginSequence);
        e.apply(Action::ChangeWord);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "X baz");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn change_to_end_with_count_does_not_repeat() {
        let mut e = editor_with("abcdef\nghijkl");
        e.apply(Action::MoveRight);
        push_digits(&mut e, &[2]);
        e.apply(Action::ChangeToEnd);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "aX");
        assert_eq!((e.row(), e.col()), (0, 1));
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

    #[test]
    fn visual_change_replaces_selection() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::ChangeSelection);
        assert_eq!(e.mode(), Mode::Insert);
        assert_eq!(e.register_text(), "bcd");
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "aXef");
        assert_eq!((e.row(), e.col()), (0, 1));
        assert_eq!(e.mode(), Mode::Normal);
    }

    #[test]
    fn visual_line_change_single_keeps_indent() {
        let mut e = editor_with("  abc\n  def");
        e.apply(Action::EnterVisualLine);
        e.apply(Action::ChangeSelection);
        assert_eq!(e.mode(), Mode::Insert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "  X");
        assert_eq!(e.text_lines()[1], "  def");
        assert_eq!((e.row(), e.col()), (0, 2));
        assert_eq!(e.mode(), Mode::Normal);
    }

    #[test]
    fn visual_line_change_multi() {
        let mut e = editor_with("abc\ndef\nghi");
        e.apply(Action::EnterVisualLine);
        e.apply(Action::MoveDown);
        e.apply(Action::ChangeSelection);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines(), vec!["X".to_string(), "ghi".to_string()]);
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.mode(), Mode::Normal);
    }

    #[test]
    fn visual_delete_past_eol_joins_next_line() {
        let mut e = editor_with("ab\ncd");
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveToLineEnd);
        e.apply(Action::DeleteSelection);
        assert_eq!(e.text_lines(), vec!["cd".to_string()]);
        assert_eq!(e.register_text(), "ab\n");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn visual_yank_past_eol_takes_newline() {
        let mut e = editor_with("ab\ncd");
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveToLineEnd);
        e.apply(Action::YankSelection);
        assert_eq!(e.text_lines(), vec!["ab".to_string(), "cd".to_string()]);
        assert_eq!(e.register_text(), "ab\n");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn visual_delete_last_line_past_eol_keeps_line() {
        let mut e = editor_with("ab\ncd");
        e.apply(Action::MoveDown);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveToLineEnd);
        e.apply(Action::DeleteSelection);
        assert_eq!(e.text_lines(), vec!["ab".to_string(), String::new()]);
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn visual_paste_over_newline_joins() {
        let mut e = editor_with("ab\ncd");
        e.apply(Action::YankToEnd);
        e.apply(Action::MoveToLineStart);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveToLineEnd);
        e.apply(Action::PasteOverSelection);
        assert_eq!(e.text_lines(), vec!["abcd".to_string()]);
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.register_text(), "ab\n");
    }

    #[test]
    fn visual_line_delete_preserves_column() {
        let mut e = editor_with("abcdef\ngh");
        e.apply(Action::EnterVisualLine);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteSelection);
        assert_eq!(e.text_lines(), vec!["gh".to_string()]);
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn visual_line_yank_moves_to_first() {
        let mut e = editor_with("abcdef");
        e.apply(Action::EnterVisualLine);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::YankSelection);
        assert_eq!(e.register_text(), "abcdef");
        assert_eq!(e.register_kind(), crate::register::RegisterKind::LineWise);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn visual_line_paste_replaces_lines() {
        let mut e = editor_with("abc\ndef\nghi");
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::YankLine);
        e.apply(Action::MoveUp);
        e.apply(Action::MoveUp);
        e.apply(Action::EnterVisualLine);
        e.apply(Action::MoveDown);
        e.apply(Action::PasteOverSelection);
        assert_eq!(e.text_lines(), vec!["ghi".to_string(), "ghi".to_string()]);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn visual_ip_fresh_selects_linewise() {
        let mut e = editor_with("a\n\nb");
        e.apply(Action::EnterVisual);
        e.apply(Action::SelectObject {
            inner: true,
            object: crate::actions::Object::Paragraph,
        });
        assert_eq!(e.mode(), Mode::VisualLine);
        assert_eq!(e.selection_range(), Some(((0, 0), (0, 1))));
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn visual_ap_fresh_takes_blank() {
        let mut e = editor_with("a\n\nb");
        e.apply(Action::EnterVisual);
        e.apply(Action::SelectObject {
            inner: false,
            object: crate::actions::Object::Paragraph,
        });
        assert_eq!(e.mode(), Mode::VisualLine);
        assert_eq!(e.selection_range(), Some(((0, 0), (1, 0))));
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn visual_ip_count_past_end_is_noop() {
        let mut e = editor_with("a\nb\nc");
        e.apply(Action::EnterVisual);
        push_digits(&mut e, &[2]);
        e.apply(Action::SelectObject {
            inner: true,
            object: crate::actions::Object::Paragraph,
        });
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 0), (0, 0))));
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn visual_ip_extends_existing_selection() {
        let mut e = editor_with("a\n\nb\n\nc");
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveDown);
        e.apply(Action::SelectObject {
            inner: true,
            object: crate::actions::Object::Paragraph,
        });
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 0), (2, 0))));
        assert_eq!((e.row(), e.col()), (2, 0));
    }

    #[test]
    fn visual_ip_trap_extends_forward() {
        let mut e = editor_with("a\n\nb\n\nc");
        e.apply(Action::EnterVisualLine);
        e.apply(Action::SelectObject {
            inner: true,
            object: crate::actions::Object::Paragraph,
        });
        assert_eq!(e.mode(), Mode::VisualLine);
        assert_eq!(e.selection_range(), Some(((0, 0), (1, 0))));
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn visual_ip_extends_backward() {
        let mut e = editor_with("a\n\nb\n\nc");
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveUp);
        e.apply(Action::MoveUp);
        e.apply(Action::SelectObject {
            inner: true,
            object: crate::actions::Object::Paragraph,
        });
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 0), (3, 0))));
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn visual_iw_in_linewise_goes_charwise() {
        let mut e = editor_with("foo bar");
        e.apply(Action::EnterVisualLine);
        e.apply(Action::SelectInnerWord);
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 0), (0, 2))));
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn visual_aw_extend_in_linewise_goes_charwise() {
        let mut e = editor_with("foo bar\nbaz qux");
        e.apply(Action::EnterVisualLine);
        e.apply(Action::MoveDown);
        e.apply(Action::SelectAroundWord);
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 0), (1, 3))));
        assert_eq!((e.row(), e.col()), (1, 3));
    }

    #[test]
    fn visual_iw_count_two() {
        let mut e = editor_with("foo bar baz");
        e.apply(Action::EnterVisual);
        push_digits(&mut e, &[2]);
        e.apply(Action::SelectInnerWord);
        assert_eq!(e.selection_range(), Some(((0, 0), (0, 3))));
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn visual_iw_count_three_deletes_two_words() {
        let mut e = editor_with("foo bar baz");
        e.apply(Action::EnterVisual);
        push_digits(&mut e, &[3]);
        e.apply(Action::SelectInnerWord);
        assert_eq!(e.selection_range(), Some(((0, 0), (0, 6))));
        e.apply(Action::DeleteSelection);
        assert_eq!(e.text_lines()[0], " baz");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn visual_iw_count_two_midword() {
        let mut e = editor_with("foo bar baz");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        push_digits(&mut e, &[2]);
        e.apply(Action::SelectInnerWord);
        assert_eq!(e.selection_range(), Some(((0, 4), (0, 7))));
        assert_eq!((e.row(), e.col()), (0, 7));
    }

    #[test]
    fn visual_aw_count_two() {
        let mut e = editor_with("foo bar baz");
        e.apply(Action::EnterVisual);
        push_digits(&mut e, &[2]);
        e.apply(Action::SelectAroundWord);
        assert_eq!(e.selection_range(), Some(((0, 0), (0, 7))));
        assert_eq!((e.row(), e.col()), (0, 7));
    }

    #[test]
    fn visual_paren_in_linewise_goes_charwise() {
        let mut e = editor_with("foo (bar)\nbaz");
        e.apply(Action::EnterVisualLine);
        e.apply(Action::SelectObject {
            inner: true,
            object: crate::actions::Object::Paren,
        });
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 5), (0, 7))));
        assert_eq!((e.row(), e.col()), (0, 7));
    }

    #[test]
    fn visual_quote_in_linewise_goes_charwise() {
        let mut e = editor_with("a 'bc' d");
        e.apply(Action::EnterVisualLine);
        e.apply(Action::SelectObject {
            inner: true,
            object: crate::actions::Object::SingleQuote,
        });
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 3), (0, 4))));
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn visual_linewise_iw_then_delete() {
        let mut e = editor_with("foo bar\nbaz qux");
        e.apply(Action::EnterVisualLine);
        e.apply(Action::MoveDown);
        e.apply(Action::SelectInnerWord);
        e.apply(Action::DeleteSelection);
        assert_eq!(e.text_lines(), vec![" qux".to_string()]);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn visual_capital_d_deletes_touched_lines() {
        let mut e = editor_with("aBcDeF");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteLines);
        assert_eq!(e.text_lines(), vec![String::new()]);
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.register_text(), "aBcDeF");
        assert_eq!(e.register_kind(), crate::register::RegisterKind::LineWise);
    }

    #[test]
    fn visual_capital_d_multiline() {
        let mut e = editor_with("aBcDeF\ngHiJkL");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteLines);
        assert_eq!(e.text_lines(), vec![String::new()]);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn visual_capital_d_ignores_count() {
        let mut e = editor_with("aBcDeF");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteLines);
        assert_eq!(e.text_lines(), vec![String::new()]);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn visual_capital_y_yanks_touched_lines() {
        let mut e = editor_with("aBcDeF\ngHiJkL");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveDown);
        e.apply(Action::YankLines);
        assert_eq!(
            e.text_lines(),
            vec!["aBcDeF".to_string(), "gHiJkL".to_string()]
        );
        assert_eq!(e.register_text(), "aBcDeF\ngHiJkL");
        assert_eq!(e.register_kind(), crate::register::RegisterKind::LineWise);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn visual_capital_c_changes_touched_lines() {
        let mut e = editor_with("aBcDeF");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::ChangeLines);
        assert_eq!(e.mode(), Mode::Insert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines(), vec!["X".to_string()]);
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.mode(), Mode::Normal);
    }

    #[test]
    fn visual_toggle_case() {
        let mut e = editor_with("aBcDeF");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::ToggleCase);
        assert_eq!(e.text_lines()[0], "abCdeF");
        assert_eq!((e.row(), e.col()), (0, 1));
        assert_eq!(e.register_text(), String::new());
    }

    #[test]
    fn visual_lowercase() {
        let mut e = editor_with("aBcDeF");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::Lowercase);
        assert_eq!(e.text_lines()[0], "abcdeF");
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn visual_uppercase() {
        let mut e = editor_with("aBcDeF");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::Uppercase);
        assert_eq!(e.text_lines()[0], "aBCDeF");
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn visual_toggle_case_multiline() {
        let mut e = editor_with("aBcDeF\ngHiJkL");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveDown);
        e.apply(Action::ToggleCase);
        assert_eq!(e.text_lines()[0], "abCdEf");
        assert_eq!(e.text_lines()[1], "GhiJkL");
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn visual_toggle_case_linewise() {
        let mut e = editor_with("aBcDeF\ngHiJkL");
        e.apply(Action::EnterVisualLine);
        e.apply(Action::MoveDown);
        e.apply(Action::ToggleCase);
        assert_eq!(e.text_lines()[0], "AbCdEf");
        assert_eq!(e.text_lines()[1], "GhIjKl");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn visual_toggle_case_point() {
        let mut e = editor_with("aBc");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::ToggleCase);
        assert_eq!(e.text_lines()[0], "abc");
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn visual_toggle_case_backward() {
        let mut e = editor_with("aBcDeF");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveLeft);
        e.apply(Action::SwapSelectionEnds);
        e.apply(Action::ToggleCase);
        assert_eq!(e.text_lines()[0], "aBCdeF");
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn visual_toggle_case_keeps_selection() {
        let mut e = editor_with("aBcDeF");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::ToggleCase);
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 1), (0, 1))));
    }

    #[test]
    fn insert_at_last_basic() {
        let mut e = editor_with("ab");
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        e.apply(Action::MoveLeft);
        e.apply(Action::InsertAtLast);
        assert_eq!(e.mode(), Mode::Insert);
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn insert_at_last_after_append() {
        let mut e = editor_with("ab");
        e.apply(Action::MoveToLineEnd);
        e.apply(Action::Append);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::InsertChar('B'));
        e.apply(Action::ExitInsert);
        e.apply(Action::MoveLeft);
        e.apply(Action::InsertAtLast);
        assert_eq!(e.mode(), Mode::Insert);
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn insert_at_last_updates() {
        let mut e = editor_with("ab");
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertChar('Y'));
        e.apply(Action::ExitInsert);
        e.apply(Action::InsertAtLast);
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn insert_at_last_fresh_enters_insert() {
        let mut e = editor_with("ab");
        e.apply(Action::MoveRight);
        e.apply(Action::InsertAtLast);
        assert_eq!(e.mode(), Mode::Insert);
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn insert_at_last_clamps_deleted_rows() {
        let mut e = editor_with("a\nb\ncc");
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        e.apply(Action::MoveUp);
        e.apply(Action::MoveUp);
        e.apply(Action::DeleteLine);
        e.apply(Action::DeleteLine);
        e.apply(Action::InsertAtLast);
        assert_eq!(e.mode(), Mode::Insert);
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn insert_at_last_ignores_count() {
        let mut e = editor_with("ab");
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        push_digits(&mut e, &[2]);
        e.apply(Action::InsertAtLast);
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn insert_at_last_empty_session() {
        let mut e = editor_with("ab");
        e.apply(Action::EnterInsert);
        e.apply(Action::ExitInsert);
        e.apply(Action::MoveRight);
        e.apply(Action::InsertAtLast);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn visual_reselect_after_escape() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::CancelVisual);
        e.apply(Action::ReselectVisual);
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 1), (0, 3))));
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn visual_reselect_after_delete() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteSelection);
        e.apply(Action::ExitVisual);
        e.apply(Action::ReselectVisual);
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 1), (0, 3))));
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn visual_reselect_after_yank() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::YankSelection);
        e.apply(Action::ExitVisual);
        e.apply(Action::ReselectVisual);
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 1), (0, 3))));
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn visual_reselect_linewise() {
        let mut e = editor_with("a\nb\nc");
        e.apply(Action::EnterVisualLine);
        e.apply(Action::MoveDown);
        e.apply(Action::CancelVisual);
        e.apply(Action::ReselectVisual);
        assert_eq!(e.mode(), Mode::VisualLine);
        assert_eq!(e.selection_range(), Some(((0, 0), (1, 1))));
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn visual_reselect_linewise_delete_clamps() {
        let mut e = editor_with("a\nb\nc");
        e.apply(Action::EnterVisualLine);
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteSelection);
        e.apply(Action::ExitVisual);
        e.apply(Action::ReselectVisual);
        assert_eq!(e.mode(), Mode::VisualLine);
        assert_eq!(e.selection_range(), Some(((0, 0), (0, 1))));
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn visual_reselect_in_visual_exchanges() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::CancelVisual);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveLeft);
        e.apply(Action::ReselectVisual);
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 1), (0, 3))));
        assert_eq!((e.row(), e.col()), (0, 3));
        e.apply(Action::ReselectVisual);
        assert_eq!(e.selection_range(), Some(((0, 2), (0, 3))));
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn visual_reselect_fresh_is_noop() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::ReselectVisual);
        assert_eq!(e.mode(), Mode::Normal);
        assert_eq!(e.selection_range(), None);
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn visual_reselect_ignores_count() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::CancelVisual);
        push_digits(&mut e, &[2]);
        e.apply(Action::ReselectVisual);
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 1), (0, 3))));
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn visual_reselect_backward_orientation() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::SwapSelectionEnds);
        e.apply(Action::CancelVisual);
        e.apply(Action::ReselectVisual);
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 1), (0, 3))));
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn visual_toggle_case_ignores_count() {
        let mut e = editor_with("aBcDeF");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        push_digits(&mut e, &[2]);
        e.apply(Action::ToggleCase);
        assert_eq!(e.text_lines()[0], "abcDeF");
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn visual_toggle_case_undoes_together() {
        let mut e = editor_with("aBcDeF\ngHiJkL");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveDown);
        e.apply(Action::ToggleCase);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "aBcDeF");
        assert_eq!(e.text_lines()[1], "gHiJkL");
    }
}
