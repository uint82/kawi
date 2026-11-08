use super::Editor;
use crate::mode::Mode;
use crate::selection::Selection;

impl Editor {
    pub fn cursor_char(&self) -> usize {
        self.buffer
            .char_index_of(self.cursor.row(), self.cursor.col())
    }

    pub fn select_char_range(&mut self, start: usize, end: usize) {
        let limit = self.buffer.total_chars();
        let start = start.min(limit);
        let end = end.min(limit);
        let start = self.buffer.position_of_char(start);
        let end = self.buffer.position_of_char(end);
        self.selection = Some(Selection::from_ends(start, end));
        self.cursor.set_position(end.0, end.1);
        self.remember_wanted_column();
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    pub(super) fn replace_selected_text(&mut self) -> bool {
        if self.mode != Mode::Insert {
            return false;
        }
        let Some(selection) = self.selection.take() else {
            return false;
        };
        let (start, end) = (selection.start(), selection.end());
        if start == end {
            self.cursor.set_position(start.0, start.1);
            return false;
        }
        self.record_boundary_at(start.0, start.1);
        let start_char = self.buffer.char_index_of(start.0, start.1);
        let end_char = self.buffer.char_index_of(end.0, end.1);
        self.buffer.delete_chars(start_char, end_char);
        self.cursor.set_position(start.0, start.1);
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::Action;
    use crate::editor::Editor;
    use kawi_buffer::Buffer;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    #[test]
    fn select_char_range_sets_cursor_and_selection() {
        let mut editor = editor_with("hello");
        editor.select_char_range(1, 4);
        assert_eq!(editor.selection_range(), Some(((0, 1), (0, 4))));
        assert_eq!(editor.cursor_char(), 4);
    }

    #[test]
    fn select_char_range_clamps_to_the_buffer() {
        let mut editor = editor_with("hello");
        editor.select_char_range(3, 99);
        assert_eq!(editor.selection_range(), Some(((0, 3), (0, 5))));
    }

    #[test]
    fn select_char_range_accepts_multiline_offsets() {
        let mut editor = editor_with("one\ntwo");
        editor.select_char_range(2, 5);
        assert_eq!(editor.selection_range(), Some(((0, 2), (1, 1))));
    }

    #[test]
    fn typing_replaces_the_selection() {
        let mut editor = editor_with("hello");
        editor.apply(Action::Append);
        editor.select_char_range(1, 4);
        editor.apply(Action::InsertChar('i'));
        assert_eq!(editor.text_lines(), ["hio"]);
        assert_eq!(editor.cursor_char(), 2);
        assert!(editor.selection_range().is_none());
    }

    #[test]
    fn backspace_deletes_the_selection_without_joining_more_text() {
        let mut editor = editor_with("hello");
        editor.apply(Action::Append);
        editor.select_char_range(1, 4);
        editor.apply(Action::DeleteBackward);
        assert_eq!(editor.text_lines(), ["ho"]);
        assert_eq!(editor.cursor_char(), 1);
    }

    #[test]
    fn delete_forward_deletes_the_selection() {
        let mut editor = editor_with("hello");
        editor.apply(Action::Append);
        editor.select_char_range(1, 4);
        editor.apply(Action::DeleteForward);
        assert_eq!(editor.text_lines(), ["ho"]);
        assert_eq!(editor.cursor_char(), 1);
    }

    #[test]
    fn newline_replaces_the_selection() {
        let mut editor = editor_with("hello");
        editor.apply(Action::Append);
        editor.select_char_range(1, 4);
        editor.apply(Action::InsertNewline);
        assert_eq!(editor.row(), 1);
    }

    #[test]
    fn paste_replaces_the_selection() {
        let mut editor = editor_with("hello");
        editor.apply(Action::Append);
        editor.select_char_range(1, 4);
        editor.paste("ey");
        assert_eq!(editor.text_lines(), ["heyo"]);
    }

    #[test]
    fn selection_replacement_is_undoable() {
        let mut editor = editor_with("hello");
        editor.apply(Action::Append);
        editor.select_char_range(1, 4);
        editor.apply(Action::InsertChar('i'));
        editor.apply(Action::ExitInsert);
        editor.apply(Action::Undo);
        assert_eq!(editor.text_lines(), ["hello"]);
    }
}
