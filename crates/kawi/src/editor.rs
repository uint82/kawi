use anyhow::Result;
use kawi_buffer::Buffer;

use crate::actions::Action;
use crate::command::CommandLine;
use crate::cursor::Cursor;
use crate::mode::Mode;

pub struct Editor {
    buffer: Buffer,
    cursor: Cursor,
    mode: Mode,
    command: CommandLine,
}

impl Editor {
    pub fn new(buffer: Buffer) -> Self {
        Self {
            buffer,
            cursor: Cursor::new(),
            mode: Mode::Normal,
            command: CommandLine::new(),
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

    pub fn save(&mut self) -> Result<()> {
        self.buffer.save()
    }

    pub fn apply(&mut self, action: Action) {
        match action {
            Action::MoveUp => self.move_up(),
            Action::MoveDown => self.move_down(),
            Action::MoveLeft => self.move_left(),
            Action::MoveRight => self.move_right(),
            Action::MoveToLineStart => self.move_to_line_start(),
            Action::MoveToLineEnd => self.move_to_line_end(),
            Action::InsertChar(c) => self.insert_char(c),
            Action::InsertNewline => self.insert_newline(),
            Action::DeleteBackward => self.delete_backward(),
            Action::DeleteForward => self.delete_forward(),
            Action::DeleteLine => self.delete_line(),
            Action::OpenLineBelow => self.open_line_below(),
            Action::EnterInsert => self.enter_insert(),
            Action::ExitInsert => self.exit_insert(),
            Action::Append => self.append(),
            Action::EnterCommand => self.enter_command(),
            Action::PushCommandChar(c) => self.push_command_char(c),
            Action::DeleteCommandChar => self.delete_command_char(),
            Action::CancelCommand => self.cancel_command(),
            Action::ExecuteCommand => self.noop(),
            Action::Save => self.save_silent(),
            Action::Quit | Action::Noop => self.noop(),
        }
        self.normalize_cursor();
    }

    fn move_up(&mut self) {
        self.cursor.move_up();
    }

    fn move_down(&mut self) {
        self.cursor.move_down();
    }

    fn move_left(&mut self) {
        self.cursor.move_left();
    }

    fn move_right(&mut self) {
        self.cursor.move_right();
    }

    fn move_to_line_start(&mut self) {
        self.cursor.set_col(0);
    }

    fn move_to_line_end(&mut self) {
        let end = self.max_col_for_row(self.cursor.row());
        self.cursor.set_col(end);
    }

    fn insert_char(&mut self, c: char) {
        self.buffer
            .insert_char(self.cursor.row(), self.cursor.col(), c);
        self.cursor.advance_col();
    }

    fn insert_newline(&mut self) {
        let row = self.cursor.row();
        self.buffer.insert_newline(row, self.cursor.col());
        self.cursor.move_to_line_start(row.saturating_add(1));
    }

    fn delete_backward(&mut self) {
        let (row, col) = self
            .buffer
            .delete_backward(self.cursor.row(), self.cursor.col());
        self.cursor.set_position(row, col);
    }

    fn delete_forward(&mut self) {
        self.buffer
            .delete_forward(self.cursor.row(), self.cursor.col());
    }

    fn delete_line(&mut self) {
        self.buffer.delete_line(self.cursor.row());
        self.cursor.set_col(0);
    }

    fn open_line_below(&mut self) {
        let row = self.cursor.row();
        let width = self.line_width(row);
        self.buffer.insert_newline(row, width);
        self.cursor.move_to_line_start(row.saturating_add(1));
        self.enter_insert();
    }

    fn enter_insert(&mut self) {
        self.mode = Mode::Insert;
    }

    fn exit_insert(&mut self) {
        self.mode = Mode::Normal;
    }

    fn append(&mut self) {
        self.cursor.move_right();
        self.enter_insert();
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
