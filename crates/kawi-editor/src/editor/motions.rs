use kawi_buffer::Buffer;

use super::Editor;
use crate::words;

impl Editor {
    fn apply_desired_column(&mut self) {
        let col = self
            .cursor
            .desired()
            .min(self.max_col_for_row(self.cursor.row()));
        self.cursor.set_col(col);
    }

    pub(super) fn jump_to(&mut self, row: usize, col: usize) {
        self.cursor.set_position(row, col);
    }

    fn jump_to_line_start(&mut self, row: usize) {
        let row = row.min(self.buffer.max_cursor_row());
        let col = self
            .buffer
            .line_cow(row)
            .map_or(0, |line| line_start_col(&line));
        self.cursor.set_position(row, col);
    }

    pub(super) fn move_down(&mut self) {
        let count = self.take_count();
        for _ in 0..count {
            let before = self.cursor.row();
            self.cursor.move_down();
            self.apply_desired_column();
            if self.cursor.row() == before {
                break;
            }
        }
    }

    pub(super) fn move_left(&mut self) {
        let count = self.take_count();
        for _ in 0..count {
            let before = (self.cursor.row(), self.cursor.col());
            self.cursor.move_left();
            if (self.cursor.row(), self.cursor.col()) == before {
                break;
            }
        }
    }

    pub(super) fn move_right(&mut self) {
        let count = self.take_count();
        for _ in 0..count {
            let before = (self.cursor.row(), self.cursor.col());
            self.cursor.move_right();
            if (self.cursor.row(), self.cursor.col()) == before {
                break;
            }
        }
    }

    pub(super) fn move_to_line_end(&mut self) {
        let count = self.take_count();
        let row = self
            .cursor
            .row()
            .saturating_add(count.saturating_sub(1))
            .min(self.buffer.max_cursor_row());
        let end = self.max_col_for_row(row);
        self.cursor.set_position(row, end);
        self.remember_line_end();
    }

    pub(super) fn move_to_line_start(&mut self) {
        self.cursor.move_to_line_start(self.cursor.row());
    }

    pub(super) fn move_to_next_line_start(&mut self) {
        let count = self.take_count();
        let mut row = self.cursor.row();
        for _ in 0..count {
            let next = row.saturating_add(1);
            if next > self.buffer.max_cursor_row() {
                break;
            }
            row = next;
        }
        if row == self.cursor.row() {
            return;
        }
        self.jump_to_line_start(row);
    }

    pub(super) fn move_to_next_word_end(&mut self) {
        self.step_word(words::next_word_end);
    }

    pub(super) fn move_to_next_word_start(&mut self) {
        self.step_word(words::next_word_start);
    }

    pub(super) fn move_to_prev_line_start(&mut self) {
        let count = self.take_count();
        let row = self.cursor.row().saturating_sub(count);
        self.jump_to_line_start(row);
    }

    pub(super) fn move_to_prev_word_start(&mut self) {
        self.step_word(words::prev_word_start);
    }

    pub(super) fn move_up(&mut self) {
        let count = self.take_count();
        for _ in 0..count {
            let before = self.cursor.row();
            self.cursor.move_up();
            self.apply_desired_column();
            if self.cursor.row() == before {
                break;
            }
        }
    }

    fn remember_line_end(&mut self) {
        if self.mode.is_insert() {
            self.cursor.set_desired(self.cursor.col());
        } else {
            self.cursor.set_desired(usize::MAX);
        }
    }

    fn step_word(&mut self, step: fn(&Buffer, usize, usize, usize) -> (usize, usize)) {
        let count = self.take_count();
        let (row, col) = step(&self.buffer, self.cursor.row(), self.cursor.col(), count);
        self.cursor.set_position(row, col);
    }
}

fn line_start_col(line: &str) -> usize {
    match line.chars().enumerate().find(|(_, ch)| !ch.is_whitespace()) {
        Some((idx, _)) => idx,
        None => line.chars().count().saturating_sub(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    fn push_digits(e: &mut Editor, digits: &[u8]) {
        for digit in digits {
            e.apply(Action::PushCount(*digit));
        }
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
    fn enter_ignores_start_column() {
        let mut e = editor_with("ab\n    cd");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveToNextLineStart);
        assert_eq!((e.row(), e.col()), (1, 4));
    }

    #[test]
    fn enter_lands_on_next_first_content() {
        let mut e = editor_with("  ab\n    cd\nef");
        e.apply(Action::MoveToNextLineStart);
        assert_eq!((e.row(), e.col()), (1, 4));
        e.apply(Action::MoveToNextLineStart);
        assert_eq!((e.row(), e.col()), (2, 0));
    }

    #[test]
    fn enter_on_empty_line_stays_at_start() {
        let mut e = editor_with("ab\n\ncd");
        e.apply(Action::MoveToNextLineStart);
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn enter_on_last_line_stays() {
        let mut e = editor_with("ab\ncd");
        e.apply(Action::MoveToNextLineStart);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveToNextLineStart);
        assert_eq!((e.row(), e.col()), (1, 1));
    }

    #[test]
    fn enter_on_whitespace_line_stops_at_end() {
        let mut e = editor_with("ab\n   \ncd");
        e.apply(Action::MoveToNextLineStart);
        assert_eq!((e.row(), e.col()), (1, 2));
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
    fn counted_hjkl_moves_repeat() {
        let mut e = editor_with("a\nb\nc");
        push_digits(&mut e, &[2]);
        e.apply(Action::MoveDown);
        assert_eq!((e.row(), e.col()), (2, 0));
        push_digits(&mut e, &[2]);
        e.apply(Action::MoveUp);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn counted_horizontal_clamps_at_edge() {
        let mut e = editor_with("abcdef");
        push_digits(&mut e, &[2]);
        e.apply(Action::MoveRight);
        assert_eq!((e.row(), e.col()), (0, 2));
        push_digits(&mut e, &[2]);
        e.apply(Action::MoveLeft);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn counted_word_motions_repeat() {
        let mut e = editor_with("foo bar baz");
        push_digits(&mut e, &[2]);
        e.apply(Action::MoveToNextWordStart);
        assert_eq!((e.row(), e.col()), (0, 8));
        push_digits(&mut e, &[2]);
        e.apply(Action::MoveToPrevWordStart);
        assert_eq!((e.row(), e.col()), (0, 0));
        push_digits(&mut e, &[2]);
        e.apply(Action::MoveToNextWordEnd);
        assert_eq!((e.row(), e.col()), (0, 6));
    }

    #[test]
    fn counted_dollar_moves_down_to_end() {
        let mut e = editor_with("ab\ncdef");
        push_digits(&mut e, &[2]);
        e.apply(Action::MoveToLineEnd);
        assert_eq!((e.row(), e.col()), (1, 3));
    }

    #[test]
    fn counted_enter_moves_down_to_start() {
        let mut e = editor_with("a\nb\nc");
        push_digits(&mut e, &[2]);
        e.apply(Action::MoveToNextLineStart);
        assert_eq!((e.row(), e.col()), (2, 0));
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
    fn line_start_end() {
        let mut e = editor_with("abc");
        e.apply(Action::MoveToLineEnd);
        assert_eq!(e.col(), 2);
        e.apply(Action::MoveToLineStart);
        assert_eq!(e.col(), 0);
    }

    #[test]
    fn minus_moves_to_prev_line_start() {
        let mut e = editor_with("ab\n   \ncd");
        e.apply(Action::MoveToNextLineStart);
        e.apply(Action::MoveToNextLineStart);
        assert_eq!((e.row(), e.col()), (2, 0));
        e.apply(Action::MoveToPrevLineStart);
        assert_eq!((e.row(), e.col()), (1, 2));
        e.apply(Action::MoveToPrevLineStart);
        assert_eq!((e.row(), e.col()), (0, 0));
        e.apply(Action::MoveToPrevLineStart);
        assert_eq!((e.row(), e.col()), (0, 0));
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
    fn word_forward_moves_to_start() {
        let mut e = editor_with("foo bar");
        e.apply(Action::MoveToNextWordStart);
        assert_eq!((e.row(), e.col()), (0, 4));
    }
}
