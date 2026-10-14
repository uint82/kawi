use anyhow::Result;
use kawi_buffer::Buffer;

use crate::actions::Action;
use crate::command::CommandLine;
use crate::cursor::Cursor;
use crate::mode::Mode;
use crate::register::{Register, RegisterKind};
use crate::selection::Selection;
use crate::words;

pub struct Editor {
    buffer: Buffer,
    cursor: Cursor,
    mode: Mode,
    command: CommandLine,
    selection: Option<Selection>,
    register: Register,
}

impl Editor {
    pub fn new(buffer: Buffer) -> Self {
        Self {
            buffer,
            cursor: Cursor::new(),
            mode: Mode::Normal,
            command: CommandLine::new(),
            selection: None,
            register: Register::new(),
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn row(&self) -> usize {
        self.cursor.row()
    }

    pub fn col(&self) -> usize {
        self.cursor.col()
    }

    pub fn is_dirty(&self) -> bool {
        self.buffer.is_dirty()
    }

    pub fn lines(&self) -> &[String] {
        self.buffer.as_slice()
    }

    pub fn filename(&self) -> &str {
        self.buffer.path.as_deref().unwrap_or("[No Name]")
    }

    pub fn command_prompt(&self) -> Option<&str> {
        if self.mode == Mode::Command {
            Some(self.command.text())
        } else {
            None
        }
    }

    pub fn command_text(&self) -> &str {
        self.command.text()
    }

    pub fn selection_range(&self) -> Option<((usize, usize), (usize, usize))> {
        self.selection
            .as_ref()
            .map(|selection| (selection.start(), selection.end()))
    }

    pub fn save(&mut self) -> Result<()> {
        self.buffer.save()
    }

    pub fn apply(&mut self, action: Action) {
        if self.needs_checkpoint(action) {
            self.record_boundary();
        }
        match action {
            Action::MoveUp => self.move_up(),
            Action::MoveDown => self.move_down(),
            Action::MoveLeft => self.move_left(),
            Action::MoveRight => self.move_right(),
            Action::MoveToLineStart => self.move_to_line_start(),
            Action::MoveToLineEnd => self.move_to_line_end(),
            Action::MoveToNextWordStart => self.move_to_next_word_start(),
            Action::MoveToPrevWordStart => self.move_to_prev_word_start(),
            Action::MoveToNextWordEnd => self.move_to_next_word_end(),
            Action::InsertChar(c) => self.insert_char(c),
            Action::InsertNewline => self.insert_newline(),
            Action::DeleteBackward => self.delete_backward(),
            Action::DeleteForward => self.delete_forward(),
            Action::DeleteLine => self.delete_line(),
            Action::OpenLineBelow => self.open_line_below(),
            Action::DeleteWord => self.delete_word(),
            Action::ChangeWord => self.change_word(),
            Action::DeleteSelection => self.delete_selection(),
            Action::YankSelection => self.yank_selection(),
            Action::YankLine => self.yank_line(),
            Action::DeleteToEnd => self.delete_to_end(),
            Action::ChangeLine => self.change_line(),
            Action::ChangeToEnd => self.change_to_end(),
            Action::PasteAfter => self.paste_after(),
            Action::Undo => self.undo(),
            Action::Redo => self.redo(),
            Action::EnterInsert => self.enter_insert(),
            Action::ExitInsert => self.exit_insert(),
            Action::Append => self.append(),
            Action::InsertBeforeSelection => self.insert_before_selection(),
            Action::InsertAfterSelection => self.insert_after_selection(),
            Action::CollapseSelection => self.collapse_selection(),
            Action::EnterVisual => self.enter_visual(),
            Action::ExitVisual => self.exit_visual(),
            Action::JumpTo(row, col) => self.jump_to(row, col),
            Action::EnterCommand => self.enter_command(),
            Action::PushCommandChar(c) => self.push_command_char(c),
            Action::DeleteCommandChar => self.delete_command_char(),
            Action::CancelCommand => self.cancel_command(),
            Action::ExecuteCommand => self.noop(),
            Action::Save => self.save_silent(),
            Action::Quit | Action::Noop => self.noop(),
        }
        self.normalize_cursor();
        self.track_head(action);
    }

    fn track_head(&mut self, action: Action) {
        if !is_tracking(action) {
            return;
        }
        self.sync_head();
    }

    fn move_up(&mut self) {
        self.cursor.move_up();
        self.apply_desired_column();
    }

    fn move_down(&mut self) {
        self.cursor.move_down();
        self.apply_desired_column();
    }

    fn move_left(&mut self) {
        self.cursor.move_left();
    }

    fn move_right(&mut self) {
        self.cursor.move_right();
    }

    fn move_to_line_start(&mut self) {
        self.cursor.move_to_line_start(self.cursor.row());
    }

    fn move_to_next_word_start(&mut self) {
        let (row, col) =
            words::next_word_start(self.buffer.as_slice(), self.cursor.row(), self.cursor.col());
        self.cursor.set_position(row, col);
    }

    fn move_to_prev_word_start(&mut self) {
        let (row, col) =
            words::prev_word_start(self.buffer.as_slice(), self.cursor.row(), self.cursor.col());
        self.cursor.set_position(row, col);
    }

    fn move_to_next_word_end(&mut self) {
        let (row, col) =
            words::next_word_end(self.buffer.as_slice(), self.cursor.row(), self.cursor.col());
        self.cursor.set_position(row, col);
    }

    fn move_to_line_end(&mut self) {
        let end = self.max_col_for_row(self.cursor.row());
        self.cursor.set_col(end);
        self.remember_line_end();
    }

    fn remember_line_end(&mut self) {
        if self.mode.is_insert() {
            self.cursor.set_desired(self.cursor.col());
        } else {
            self.cursor.set_desired(usize::MAX);
        }
    }

    fn apply_desired_column(&mut self) {
        let col = self
            .cursor
            .desired()
            .min(self.max_col_for_row(self.cursor.row()));
        self.cursor.set_col(col);
    }

    fn insert_char(&mut self, c: char) {
        self.record_maybe();
        self.buffer
            .insert_char(self.cursor.row(), self.cursor.col(), c);
        self.cursor.advance_col();
    }

    fn insert_newline(&mut self) {
        let row = self.cursor.row();
        self.insert_newline_at(row, self.cursor.col());
    }

    fn insert_newline_at(&mut self, row: usize, col: usize) {
        self.record_maybe();
        self.buffer.insert_newline(row, col);
        self.cursor.move_to_line_start(row.saturating_add(1));
    }

    fn delete_backward(&mut self) {
        self.record_maybe();
        let (row, col) = self
            .buffer
            .delete_backward(self.cursor.row(), self.cursor.col());
        self.cursor.set_position(row, col);
    }

    fn delete_forward(&mut self) {
        self.record_maybe();
        self.buffer
            .delete_forward(self.cursor.row(), self.cursor.col());
    }

    fn delete_line(&mut self) {
        self.yank_current_line();
        self.buffer.delete_line(self.cursor.row());
        self.cursor.set_col(0);
    }

    fn delete_word(&mut self) {
        let Some(((start_row, start_col), (end_row, end_col))) =
            words::dw_range(self.buffer.as_slice(), self.cursor.row(), self.cursor.col())
        else {
            return;
        };
        self.record_boundary_at(start_row, start_col);
        self.yank_range(start_row, start_col, end_row, end_col);
        self.buffer
            .delete_range(start_row, start_col, end_row, end_col);
        self.cursor.set_position(start_row, start_col);
    }

    fn change_word(&mut self) {
        let Some(((start_row, start_col), (end_row, end_col))) =
            words::cw_range(self.buffer.as_slice(), self.cursor.row(), self.cursor.col())
        else {
            return;
        };
        self.record_boundary_at(start_row, start_col);
        self.yank_range(start_row, start_col, end_row, end_col);
        self.buffer
            .delete_range(start_row, start_col, end_row, end_col);
        self.cursor.set_position(start_row, start_col);
        self.switch_to_insert();
    }

    fn yank_current_line(&mut self) {
        let text = self.buffer.line_text(self.cursor.row());
        self.register.set_lines(vec![text]);
    }

    fn yank_line(&mut self) {
        self.yank_current_line();
    }

    fn delete_to_end(&mut self) {
        if self.line_is_empty_at_cursor() {
            return;
        }
        self.record_boundary();
        self.yank_to_end();
        self.buffer.delete_range(
            self.cursor.row(),
            self.cursor.col(),
            self.cursor.row(),
            self.line_width(self.cursor.row()).saturating_sub(1),
        );
    }

    fn line_is_empty_at_cursor(&self) -> bool {
        let width = self.line_width(self.cursor.row());
        width == 0 || self.cursor.col() >= width
    }

    fn yank_to_end(&mut self) {
        let row = self.cursor.row();
        let text = self.buffer.range_text(
            row,
            self.cursor.col(),
            row,
            self.line_width(row).saturating_sub(1),
        );
        self.register.set_text(text);
    }

    fn change_line(&mut self) {
        self.yank_current_line();
        if self.line_is_empty_at_cursor() {
            self.enter_insert();
            return;
        }
        self.record_boundary();
        self.buffer.clear_line(self.cursor.row());
        self.cursor.set_col(0);
        self.switch_to_insert();
    }

    fn change_to_end(&mut self) {
        if self.line_is_empty_at_cursor() {
            self.enter_insert();
            return;
        }
        self.record_boundary();
        self.yank_to_end();
        self.buffer.delete_range(
            self.cursor.row(),
            self.cursor.col(),
            self.cursor.row(),
            self.line_width(self.cursor.row()).saturating_sub(1),
        );
        self.clamp_col_normal();
        self.switch_to_insert();
    }

    fn clamp_col_normal(&mut self) {
        let width = self.line_width(self.cursor.row());
        self.cursor
            .set_col(self.cursor.col().min(width.saturating_sub(1)));
    }

    fn delete_selection(&mut self) {
        let ((start_row, start_col), (end_row, end_col)) = self.operator_range();
        self.record_boundary_at(start_row, start_col);
        self.yank_range(start_row, start_col, end_row, end_col);
        self.buffer
            .delete_range(start_row, start_col, end_row, end_col);
        self.collapse_to(start_row, start_col);
        self.cursor.set_position(start_row, start_col);
    }

    fn yank_selection(&mut self) {
        let ((start_row, start_col), (end_row, end_col)) = self.operator_range();
        self.yank_range(start_row, start_col, end_row, end_col);
        self.cursor.set_position(start_row, start_col);
    }

    fn operator_range(&self) -> ((usize, usize), (usize, usize)) {
        match self.selection_range() {
            Some(range) => range,
            None => (
                (self.cursor.row(), self.cursor.col()),
                (self.cursor.row(), self.cursor.col()),
            ),
        }
    }

    fn collapse_to(&mut self, row: usize, col: usize) {
        self.selection = Some(Selection::new(row, col));
    }

    fn yank_range(&mut self, start_row: usize, start_col: usize, end_row: usize, end_col: usize) {
        let text = self
            .buffer
            .range_text(start_row, start_col, end_row, end_col);
        self.register.set_text(text);
    }

    fn paste_after(&mut self) {
        if self.register.is_empty() {
            return;
        }
        self.record_boundary();
        match self.register.kind() {
            RegisterKind::CharWise => self.paste_inline(),
            RegisterKind::LineWise => self.paste_lines(),
        }
    }

    fn paste_inline(&mut self) {
        let text = self.register.text().to_string();
        let (row, col) = self
            .buffer
            .paste_after(self.cursor.row(), self.cursor.col(), &text);
        self.cursor.set_position(row, col);
    }

    fn paste_lines(&mut self) {
        let lines: Vec<String> = self.register.text().lines().map(str::to_string).collect();
        let row = self.buffer.paste_lines_below(self.cursor.row(), &lines);
        self.cursor.set_position(row, self.first_non_blank_of(row));
    }

    fn first_non_blank_of(&self, row: usize) -> usize {
        self.buffer
            .line(row)
            .map_or(0, kawi_buffer::first_non_blank)
    }

    fn open_line_below(&mut self) {
        let row = self.cursor.row();
        let width = self.line_width(row);
        self.buffer.begin_change();
        self.insert_newline_at(row, width);
        self.switch_to_insert();
    }

    fn undo(&mut self) {
        if let Some((row, col)) = self.buffer.undo(self.cursor.row(), self.cursor.col()) {
            self.cursor.set_position(row, col);
        }
    }

    fn redo(&mut self) {
        self.buffer
            .redo_keep_cursor(self.cursor.row(), self.cursor.col());
    }

    fn record_boundary(&mut self) {
        self.buffer.checkpoint(self.cursor.row(), self.cursor.col());
    }

    fn record_boundary_at(&mut self, row: usize, col: usize) {
        self.buffer.checkpoint(row, col);
    }

    fn record_maybe(&mut self) {
        self.buffer
            .maybe_checkpoint(self.cursor.row(), self.cursor.col());
    }

    fn needs_checkpoint(&self, action: Action) -> bool {
        self.mode == Mode::Normal && matches!(action, Action::DeleteForward | Action::DeleteLine)
    }

    fn enter_insert(&mut self) {
        self.buffer.begin_change();
        self.switch_to_insert();
    }

    fn switch_to_insert(&mut self) {
        self.mode = Mode::Insert;
    }

    fn exit_insert(&mut self) {
        if self.mode == Mode::Insert {
            self.buffer.end_change();
            self.retreat_after_insert();
        }
        self.mode = Mode::Normal;
    }

    fn retreat_after_insert(&mut self) {
        if self.cursor.col() > 0 {
            self.cursor.move_left();
        }
    }

    fn append(&mut self) {
        self.cursor.move_right();
        self.enter_insert();
    }

    fn insert_before_selection(&mut self) {
        if let Some(((row, col), _)) = self.selection_range() {
            self.cursor.set_position(row, col);
        }
        self.exit_visual();
        self.enter_insert();
    }

    fn insert_after_selection(&mut self) {
        if let Some((_, (row, col))) = self.selection_range() {
            self.cursor.set_position(row, col);
            self.cursor.move_right();
        }
        self.exit_visual();
        self.enter_insert();
    }

    fn collapse_selection(&mut self) {
        if self.selection.is_some() {
            self.collapse_to(self.cursor.row(), self.cursor.col());
        }
    }

    fn enter_visual(&mut self) {
        self.selection = Some(Selection::new(self.cursor.row(), self.cursor.col()));
        self.mode = Mode::Visual;
    }

    fn exit_visual(&mut self) {
        self.selection = None;
        self.mode = Mode::Normal;
    }

    fn jump_to(&mut self, row: usize, col: usize) {
        self.cursor.set_position(row, col);
    }

    fn sync_head(&mut self) {
        if let Some(selection) = self.selection.as_mut() {
            selection.set_head(self.cursor.row(), self.cursor.col());
        }
    }

    fn enter_command(&mut self) {
        self.mode = Mode::Command;
        self.command.clear();
    }

    fn push_command_char(&mut self, c: char) {
        if self.mode == Mode::Command {
            self.command.push(c);
        }
    }

    fn delete_command_char(&mut self) {
        if self.mode == Mode::Command {
            self.command.pop();
        }
    }

    fn cancel_command(&mut self) {
        self.mode = Mode::Normal;
        self.command.clear();
    }

    fn save_silent(&mut self) {
        let _ = self.buffer.save();
    }

    fn noop(&mut self) {}

    fn normalize_cursor(&mut self) {
        self.clamp_row();
        self.clamp_col();
    }

    fn clamp_row(&mut self) {
        let max_row = self.buffer.len().saturating_sub(1);
        self.cursor.set_row(self.cursor.row().min(max_row));
    }

    fn clamp_col(&mut self) {
        let max_col = self.max_col_for_row(self.cursor.row());
        self.cursor.set_col(self.cursor.col().min(max_col));
    }

    fn max_col_for_row(&self, row: usize) -> usize {
        let width = self.line_width(row);
        if self.mode.is_insert() {
            width
        } else {
            width.saturating_sub(1)
        }
    }

    fn line_width(&self, row: usize) -> usize {
        self.buffer.line(row).map_or(0, char_count)
    }
}

fn char_count(text: &str) -> usize {
    text.chars().count()
}

fn is_tracking(action: Action) -> bool {
    matches!(
        action,
        Action::MoveUp
            | Action::MoveDown
            | Action::MoveLeft
            | Action::MoveRight
            | Action::MoveToLineStart
            | Action::MoveToLineEnd
            | Action::MoveToNextWordStart
            | Action::MoveToPrevWordStart
            | Action::MoveToNextWordEnd
            | Action::JumpTo(_, _)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    #[test]
    fn starts_in_normal() {
        let e = editor_with("hi");
        assert_eq!(e.mode(), Mode::Normal);
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
    fn exit_insert_steps_back() {
        let mut e = editor_with("abcdef");
        e.apply(Action::EnterInsert);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::ExitInsert);
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
    fn exit_insert_at_start_stays() {
        let mut e = editor_with("abc");
        e.apply(Action::EnterInsert);
        e.apply(Action::ExitInsert);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn insert_char_advances() {
        let mut e = editor_with("ac");
        e.apply(Action::MoveRight);
        e.apply(Action::InsertChar('b'));
        assert_eq!(e.lines()[0], "abc");
        assert_eq!(e.col(), 2);
    }

    #[test]
    fn hjkl_move() {
        let mut e = editor_with("ab\ncd");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveDown);
        assert_eq!((e.row(), e.col()), (1, 1));
        e.apply(Action::MoveLeft);
        e.apply(Action::MoveUp);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn line_start_end() {
        let mut e = editor_with("abc");
        e.apply(Action::MoveToLineEnd);
        assert_eq!(e.col(), 2);
        e.apply(Action::MoveToLineStart);
        assert_eq!(e.col(), 0);
    }

    #[test]
    fn vertical_keeps_goal_column() {
        let mut e = editor_with("0123456789\nabcd\nabcdefghi\n0123456789AB");
        for _ in 0..9 {
            e.apply(Action::MoveRight);
        }
        assert_eq!((e.row(), e.col()), (0, 9));
        e.apply(Action::MoveDown);
        assert_eq!((e.row(), e.col()), (1, 3));
        e.apply(Action::MoveDown);
        assert_eq!((e.row(), e.col()), (2, 8));
        e.apply(Action::MoveDown);
        assert_eq!((e.row(), e.col()), (3, 9));
    }

    #[test]
    fn vertical_up_restores_goal_column() {
        let mut e = editor_with("0123456789\nabcd\nabcdefghi\n0123456789AB");
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        for _ in 0..9 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::MoveUp);
        assert_eq!((e.row(), e.col()), (2, 8));
        e.apply(Action::MoveUp);
        assert_eq!((e.row(), e.col()), (1, 3));
        e.apply(Action::MoveUp);
        assert_eq!((e.row(), e.col()), (0, 9));
    }

    #[test]
    fn line_end_sticks_to_ends() {
        let mut e = editor_with("0123456789\nabcd\nabcdefghi\n0123456789AB");
        e.apply(Action::MoveToLineEnd);
        e.apply(Action::MoveDown);
        assert_eq!((e.row(), e.col()), (1, 3));
        e.apply(Action::MoveDown);
        assert_eq!((e.row(), e.col()), (2, 8));
        e.apply(Action::MoveDown);
        assert_eq!((e.row(), e.col()), (3, 11));
    }

    #[test]
    fn line_start_clears_sticky_end() {
        let mut e = editor_with("0123456789\nabcd\nabcdefghi\n0123456789AB");
        e.apply(Action::MoveToLineEnd);
        e.apply(Action::MoveToLineStart);
        e.apply(Action::MoveDown);
        assert_eq!((e.row(), e.col()), (1, 0));
        e.apply(Action::MoveDown);
        assert_eq!((e.row(), e.col()), (2, 0));
        e.apply(Action::MoveDown);
        assert_eq!((e.row(), e.col()), (3, 0));
    }

    #[test]
    fn normal_stays_on_last_char() {
        let mut e = editor_with("abcdef");
        for _ in 0..10 {
            e.apply(Action::MoveRight);
        }
        assert_eq!(e.col(), 5);
    }

    #[test]
    fn insert_reaches_line_end() {
        let mut e = editor_with("abcdef");
        e.apply(Action::EnterInsert);
        for _ in 0..10 {
            e.apply(Action::MoveRight);
        }
        assert_eq!(e.col(), 6);
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
    fn empty_line_stays_zero() {
        let mut e = editor_with("");
        e.apply(Action::MoveRight);
        assert_eq!(e.col(), 0);
        e.apply(Action::EnterInsert);
        e.apply(Action::MoveRight);
        assert_eq!(e.col(), 0);
    }

    #[test]
    fn delete_forward_removes() {
        let mut e = editor_with("ab");
        e.apply(Action::DeleteForward);
        assert_eq!(e.lines()[0], "b");
    }

    #[test]
    fn delete_line_removes_row() {
        let mut e = editor_with("a\nb\nc");
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteLine);
        assert_eq!(e.lines().len(), 2);
        assert_eq!(e.lines()[1], "c");
    }

    #[test]
    fn open_line_below_enters_insert() {
        let mut e = editor_with("ab");
        e.apply(Action::OpenLineBelow);
        assert_eq!(e.lines().len(), 2);
        assert_eq!((e.row(), e.col()), (1, 0));
        assert_eq!(e.mode(), Mode::Insert);
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
    fn command_backspace_shrinks() {
        let mut e = editor_with("ab");
        e.apply(Action::EnterCommand);
        e.apply(Action::PushCommandChar('w'));
        e.apply(Action::DeleteCommandChar);
        assert_eq!(e.command_prompt(), Some(""));
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
    fn insert_session_undoes_together() {
        let mut e = editor_with("ab");
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::InsertChar('Y'));
        e.apply(Action::InsertChar('Z'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        assert_eq!(e.lines()[0], "ab");
    }

    #[test]
    fn empty_insert_leaves_no_history() {
        let mut e = editor_with("ab");
        e.apply(Action::EnterInsert);
        e.apply(Action::ExitInsert);
        let before = e.lines()[0].to_string();
        e.apply(Action::Undo);
        assert_eq!(e.lines()[0], before);
    }

    #[test]
    fn open_line_session_undoes_together() {
        let mut e = editor_with("ab");
        e.apply(Action::OpenLineBelow);
        e.apply(Action::InsertChar('H'));
        e.apply(Action::InsertChar('i'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        assert_eq!(e.lines().len(), 1);
        assert_eq!(e.lines()[0], "ab");
    }

    #[test]
    fn redo_restores_undone_session() {
        let mut e = editor_with("ab");
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        e.apply(Action::Redo);
        assert_eq!(e.lines()[0], "Xab");
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
        assert_eq!(e.lines()[0], "b");
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
        assert_eq!(e.lines().len(), 2);
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
        assert_eq!(e.lines()[0], "aa");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn undo_delete_returns_cursor() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteForward);
        assert_eq!(e.lines()[0], "abdef");
        e.apply(Action::Undo);
        assert_eq!(e.lines()[0], "abcdef");
        assert_eq!((e.row(), e.col()), (0, 2));
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
        assert_eq!(e.lines()[2], "Hi");
        assert_eq!((e.row(), e.col()), (1, 0));
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
    fn visual_delete_cuts_range() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteSelection);
        assert_eq!(e.lines()[0], "aef");
        assert_eq!((e.row(), e.col()), (0, 1));
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 1), (0, 1))));
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
        assert_eq!(e.lines()[0], "abcdef");
    }

    #[test]
    fn undo_multiline_delete_returns_to_start() {
        let mut e = editor_with("ab\ncd");
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteSelection);
        assert_eq!(e.lines().len(), 1);
        e.apply(Action::Undo);
        assert_eq!(e.lines().len(), 2);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn yank_line_pastes_below() {
        let mut e = editor_with("aa\nbb\ncc");
        e.apply(Action::MoveDown);
        e.apply(Action::YankLine);
        e.apply(Action::PasteAfter);
        assert_eq!(e.lines().len(), 4);
        assert_eq!(e.lines()[2], "bb");
        assert_eq!((e.row(), e.col()), (2, 0));
    }

    #[test]
    fn delete_line_yanks_for_paste() {
        let mut e = editor_with("aa\nbb");
        e.apply(Action::DeleteLine);
        e.apply(Action::PasteAfter);
        assert_eq!(e.lines()[0], "bb");
        assert_eq!(e.lines()[1], "aa");
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn delete_to_end_keeps_head() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteToEnd);
        assert_eq!(e.lines()[0], "ab");
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn delete_to_end_empty_is_noop() {
        let mut e = editor_with("");
        e.apply(Action::DeleteToEnd);
        e.apply(Action::Undo);
        assert_eq!(e.lines()[0], "");
    }

    #[test]
    fn change_line_clears_and_groups_typing() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::ChangeLine);
        assert_eq!(e.lines()[0], "");
        assert_eq!(e.mode(), Mode::Insert);
        e.apply(Action::InsertChar('x'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        assert_eq!(e.lines()[0], "abcdef");
    }

    #[test]
    fn change_to_end_positions_cursor() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::ChangeToEnd);
        assert_eq!(e.lines()[0], "ab");
        assert_eq!((e.row(), e.col()), (0, 1));
        assert_eq!(e.mode(), Mode::Insert);
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
    fn visual_yank_keeps_text() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::YankSelection);
        assert_eq!(e.lines()[0], "abcdef");
        assert_eq!((e.row(), e.col()), (0, 1));
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 1), (0, 3))));
        assert!(!e.is_dirty());
    }

    #[test]
    fn paste_after_deleted_text() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteSelection);
        e.apply(Action::PasteAfter);
        assert_eq!(e.lines()[0], "aebcdf");
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn paste_empty_register_keeps_history() {
        let mut e = editor_with("ab");
        e.apply(Action::PasteAfter);
        e.apply(Action::Undo);
        assert_eq!(e.lines()[0], "ab");
    }

    #[test]
    fn word_forward_moves_to_start() {
        let mut e = editor_with("foo bar");
        e.apply(Action::MoveToNextWordStart);
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn word_backward_moves_to_start() {
        let mut e = editor_with("foo bar");
        e.apply(Action::MoveToLineEnd);
        e.apply(Action::MoveToPrevWordStart);
        assert_eq!((e.row(), e.col()), (0, 4));
        e.apply(Action::MoveToPrevWordStart);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn word_end_moves_to_end() {
        let mut e = editor_with("foo bar");
        e.apply(Action::MoveToNextWordEnd);
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn delete_word_deletes_forward() {
        let mut e = editor_with("foo bar");
        e.apply(Action::DeleteWord);
        assert_eq!(e.lines()[0], "bar");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_word_last_word_trims() {
        let mut e = editor_with("foo bar");
        for _ in 0..4 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteWord);
        assert_eq!(e.lines()[0], "foo ");
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn delete_word_undoes_together() {
        let mut e = editor_with("foo bar");
        e.apply(Action::DeleteWord);
        e.apply(Action::Undo);
        assert_eq!(e.lines()[0], "foo bar");
    }

    #[test]
    fn change_word_keeps_trailing_space() {
        let mut e = editor_with("foo bar");
        e.apply(Action::ChangeWord);
        assert_eq!(e.lines()[0], " bar");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn change_word_groups_typing() {
        let mut e = editor_with("foo bar");
        e.apply(Action::ChangeWord);
        e.apply(Action::InsertChar('b'));
        e.apply(Action::InsertChar('a'));
        e.apply(Action::InsertChar('z'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        assert_eq!(e.lines()[0], "foo bar");
    }

    #[test]
    fn jump_clamps_to_last_char_in_normal() {
        let mut e = editor_with("abcdef");
        e.apply(Action::JumpTo(0, 20));
        assert_eq!((e.row(), e.col()), (0, 5));
    }

    #[test]
    fn jump_reaches_end_in_insert() {
        let mut e = editor_with("abcdef");
        e.apply(Action::EnterInsert);
        e.apply(Action::JumpTo(0, 20));
        assert_eq!((e.row(), e.col()), (0, 6));
    }

    #[test]
    fn jump_remembers_goal_column() {
        let mut e = editor_with("0123456789\nabcd");
        e.apply(Action::JumpTo(1, 2));
        e.apply(Action::MoveUp);
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn append_steps_right_and_inserts() {
        let mut e = editor_with("ab");
        e.apply(Action::Append);
        assert_eq!(e.mode(), Mode::Insert);
        assert_eq!((e.row(), e.col()), (0, 1));
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
    fn append_on_empty_stays_zero() {
        let mut e = editor_with("");
        e.apply(Action::Append);
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.mode(), Mode::Insert);
    }
}
