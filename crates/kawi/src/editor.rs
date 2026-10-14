use anyhow::Result;
use kawi_buffer::Buffer;

use crate::actions::Action;
use crate::cursor::Cursor;
use crate::mode::Mode;

pub struct Editor {
    buffer: Buffer,
    cursor: Cursor,
    mode: Mode,
}

impl Editor {
    pub fn new(buffer: Buffer) -> Self {
        Self {
            buffer,
            cursor: Cursor::new(),
            mode: Mode::Normal,
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
            Action::EnterInsert => self.enter_insert(),
            Action::ExitInsert => self.exit_insert(),
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
        let width = self.line_width(self.cursor.row());
        self.cursor.set_col(width);
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

    fn enter_insert(&mut self) {
        self.mode = Mode::Insert;
    }

    fn exit_insert(&mut self) {
        self.mode = Mode::Normal;
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
        let width = self.line_width(self.cursor.row());
        self.cursor.set_col(self.cursor.col().min(width));
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
        assert_eq!(e.col(), 3);
        e.apply(Action::MoveToLineStart);
        assert_eq!(e.col(), 0);
    }

    #[test]
    fn delete_forward_removes() {
        let mut e = editor_with("ab");
        e.apply(Action::DeleteForward);
        assert_eq!(e.lines()[0], "b");
    }
}
