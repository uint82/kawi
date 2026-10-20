use super::Editor;
use crate::actions::Action;

const CHANGE_LIST_SIZE: usize = 100;
const CHANGE_COLUMN_DISTANCE: usize = 79;

impl Editor {
    pub(super) fn change_origin(&self) -> (usize, usize) {
        if let Some(origin) = self.change_origin_override {
            return origin;
        }
        if self.mode.is_visual() {
            if let Some(selection) = self.selection.as_ref() {
                return selection.start();
            }
        }
        (self.cursor.row(), self.cursor.col())
    }

    pub(super) fn breaks_insert_run(action: Action) -> bool {
        matches!(
            action,
            Action::InsertNewline
                | Action::InsertTab
                | Action::DeleteBackward
                | Action::DeleteForward
                | Action::MoveUp
                | Action::MoveDown
                | Action::MoveLeft
                | Action::MoveRight
                | Action::MoveToLineStart
                | Action::MoveToLineEnd
        )
    }

    pub(super) fn note_buffer_change(
        &mut self,
        origin: (usize, usize),
        insert: bool,
        insert_entry: bool,
    ) {
        let origin = self.change_origin_override.unwrap_or(origin);
        if insert_entry {
            self.start_insert_entry(origin);
        } else if insert {
            self.note_insert_change(origin);
        } else {
            self.push_change_entry(origin, true);
        }
    }

    fn start_insert_entry(&mut self, origin: (usize, usize)) {
        let origin = self.insert_start.unwrap_or(origin);
        if self.insert_session_origin.is_none() {
            self.push_change_entry(origin, true);
        } else {
            self.update_last_change(origin);
        }
        self.insert_session_origin = Some(origin);
        self.insert_run_origin = Some(origin);
        self.insert_run_active = false;
    }

    fn note_insert_change(&mut self, origin: (usize, usize)) {
        if self.insert_session_origin.is_none() {
            self.push_change_entry(origin, true);
            self.insert_session_origin = Some(origin);
            self.insert_run_origin = Some(origin);
            self.insert_run_active = true;
            return;
        }
        let origin = if self.insert_run_active {
            self.insert_run_origin.unwrap_or(origin)
        } else {
            self.insert_run_origin = Some(origin);
            self.insert_run_active = true;
            origin
        };
        self.update_last_change(origin);
    }

    fn update_last_change(&mut self, origin: (usize, usize)) {
        if let Some(last) = self.change_list.last_mut() {
            *last = origin;
        }
        self.change_index = self.change_list.len();
    }

    fn push_change_entry(&mut self, origin: (usize, usize), new_block: bool) {
        let add = match self.change_list.last() {
            None => true,
            Some(&(row, col)) => {
                new_block && (row != origin.0 || col.abs_diff(origin.1) > CHANGE_COLUMN_DISTANCE)
            }
        };
        if add {
            if self.change_list.len() == CHANGE_LIST_SIZE {
                self.change_list.remove(0);
                self.change_index = self.change_index.saturating_sub(1);
            }
            self.change_list.push(origin);
        } else if let Some(last) = self.change_list.last_mut() {
            *last = origin;
        }
        self.change_index = self.change_list.len();
    }

    pub(super) fn older_change(&mut self) {
        self.change_step(true);
    }

    pub(super) fn newer_change(&mut self) {
        self.change_step(false);
    }

    fn change_step(&mut self, older: bool) {
        let count = self.take_count();
        let len = self.change_list.len();
        if len == 0 {
            return;
        }
        if older {
            if self.change_index == 0 {
                return;
            }
            self.change_index = self.change_index.saturating_sub(count);
        } else {
            if self.change_index == len - 1 {
                return;
            }
            self.change_index = self.change_index.saturating_add(count).min(len - 1);
        }
        let (row, col) = self.change_list[self.change_index];
        let row = row.min(self.buffer.line_count().saturating_sub(1));
        let col = col.min(self.max_col_for_row(row));
        self.cursor.set_position(row, col);
        self.remember_wanted_column();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_buffer::Buffer;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    #[test]
    fn separate_lines_create_entries() {
        let mut editor = editor_with("aaa\nbbb\nccc\n");
        editor.apply(Action::DeleteForward);
        editor.apply(Action::MoveDown);
        editor.apply(Action::DeleteForward);
        editor.apply(Action::MoveDown);
        editor.apply(Action::DeleteForward);
        editor.apply(Action::OlderChange);
        assert_eq!((editor.row(), editor.col()), (2, 0));
        editor.apply(Action::OlderChange);
        assert_eq!((editor.row(), editor.col()), (1, 0));
        editor.apply(Action::OlderChange);
        assert_eq!((editor.row(), editor.col()), (0, 0));
        editor.apply(Action::OlderChange);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn newer_change_walks_forward_and_stops() {
        let mut editor = editor_with("aaa\nbbb\nccc\n");
        editor.apply(Action::DeleteForward);
        editor.apply(Action::MoveDown);
        editor.apply(Action::DeleteForward);
        editor.apply(Action::OlderChange);
        editor.apply(Action::OlderChange);
        editor.apply(Action::NewerChange);
        assert_eq!((editor.row(), editor.col()), (1, 0));
        editor.apply(Action::NewerChange);
        assert_eq!((editor.row(), editor.col()), (1, 0));
    }

    #[test]
    fn counted_older_change_skips_entries() {
        let mut editor = editor_with("aaa\nbbb\nccc\nddd\n");
        for _ in 0..3 {
            editor.apply(Action::DeleteForward);
            editor.apply(Action::MoveDown);
        }
        editor.apply(Action::PushCount(2));
        editor.apply(Action::OlderChange);
        assert_eq!((editor.row(), editor.col()), (1, 0));
    }

    #[test]
    fn changes_on_same_line_nearby_update_one_entry() {
        let mut editor = editor_with("abcdefghij\n");
        editor.apply(Action::DeleteForward);
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::DeleteForward);
        editor.apply(Action::OlderChange);
        assert_eq!((editor.row(), editor.col()), (0, 3));
    }

    #[test]
    fn insert_session_records_first_change() {
        let mut editor = editor_with("abc\ndef\n");
        editor.apply(Action::EnterInsert);
        editor.apply(Action::MoveRight);
        editor.apply(Action::InsertChar('X'));
        editor.apply(Action::InsertChar('Y'));
        editor.apply(Action::ExitInsert);
        editor.apply(Action::OlderChange);
        assert_eq!((editor.row(), editor.col()), (0, 1));
    }

    #[test]
    fn insert_newline_moves_entry_to_split_point() {
        let mut editor = editor_with("abcdef\n");
        editor.apply(Action::EnterInsert);
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::InsertNewline);
        editor.apply(Action::ExitInsert);
        editor.apply(Action::OlderChange);
        assert_eq!((editor.row(), editor.col()), (0, 1));
    }

    #[test]
    fn insert_after_newline_uses_new_line() {
        let mut editor = editor_with("abcdef\n");
        editor.apply(Action::EnterInsert);
        editor.apply(Action::InsertNewline);
        editor.apply(Action::InsertChar('X'));
        editor.apply(Action::ExitInsert);
        editor.apply(Action::OlderChange);
        assert_eq!((editor.row(), editor.col()), (1, 0));
    }

    #[test]
    fn insert_move_starts_new_run() {
        let mut editor = editor_with("abcdef\n");
        editor.apply(Action::EnterInsert);
        editor.apply(Action::InsertChar('a'));
        editor.apply(Action::MoveLeft);
        editor.apply(Action::InsertChar('X'));
        editor.apply(Action::ExitInsert);
        editor.apply(Action::OlderChange);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn empty_changelist_is_no_op() {
        let mut editor = editor_with("abc\n");
        editor.apply(Action::OlderChange);
        editor.apply(Action::NewerChange);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }
}
