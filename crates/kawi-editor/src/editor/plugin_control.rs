use kawi_buffer::char_offset_at;

use super::Editor;
use crate::mode::Mode;
use crate::selection::Selection;

impl Editor {
    pub fn set_cursor_position(&mut self, line: usize, byte_column: usize) -> bool {
        let row = line.min(self.buffer.max_cursor_row());
        let column = self.char_column(row, byte_column);
        if self.mode.is_visual() {
            if let Some(selection) = self.selection.as_mut() {
                selection.set_head(row, column);
            }
        }
        self.jump_to(row, column);
        true
    }

    pub fn set_selection_ends(&mut self, start: (usize, usize), end: (usize, usize)) -> bool {
        let start = self.clamp_position(start);
        let end = self.clamp_position(end);
        self.selection = Some(Selection::from_ends(start, end));
        self.cursor.set_position(end.0, end.1);
        self.mode = Mode::Visual;
        true
    }

    pub fn clear_visual_selection(&mut self) -> bool {
        if self.mode.is_visual() {
            self.cancel_visual();
        } else {
            self.selection = None;
        }
        true
    }

    pub fn set_editor_mode(&mut self, mode: Mode) -> bool {
        match mode {
            Mode::Normal => self.enter_normal_mode(),
            Mode::Insert => self.enter_insert_mode(),
            Mode::Replace => self.enter_replace_mode(),
            Mode::Visual => self.enter_selection_mode(Mode::Visual),
            Mode::VisualLine => self.enter_selection_mode(Mode::VisualLine),
            Mode::VisualBlock => self.enter_selection_mode(Mode::VisualBlock),
            Mode::Command | Mode::Search => return false,
        }
        true
    }

    fn enter_normal_mode(&mut self) {
        if self.mode.is_insert() {
            self.exit_insert();
        }
        if self.mode.is_visual() {
            self.cancel_visual();
        }
        self.mode = Mode::Normal;
    }

    fn enter_insert_mode(&mut self) {
        if self.mode.is_visual() {
            self.exit_visual();
        }
        if self.mode.is_insert() {
            self.exit_insert();
        }
        self.enter_insert();
    }

    fn enter_replace_mode(&mut self) {
        if self.mode.is_visual() {
            self.exit_visual();
        }
        if self.mode.is_insert() {
            self.exit_insert();
        }
        self.enter_replace();
    }

    fn enter_selection_mode(&mut self, mode: Mode) {
        if self.mode.is_insert() {
            self.exit_insert();
        }
        self.mode = mode;
        if self.selection.is_none() {
            self.selection = Some(Selection::new(self.cursor.row(), self.cursor.col()));
        }
    }

    fn char_column(&self, row: usize, byte_column: usize) -> usize {
        let text = self.buffer.snapshot();
        let line_start = text.line_to_char(row);
        let line_bytes = self.buffer.line_cow(row).map_or(0, |line| line.len());
        let mut column = byte_column.min(line_bytes);
        while column > 0 {
            if let Ok(offset) = char_offset_at(&text, row, column) {
                return offset.saturating_sub(line_start);
            }
            column -= 1;
        }
        0
    }

    fn clamp_position(&self, position: (usize, usize)) -> (usize, usize) {
        let row = position.0.min(self.buffer.max_cursor_row());
        (row, position.1.min(self.buffer.line_len(row)))
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
    fn cursor_uses_utf8_byte_columns() {
        let mut editor = editor_with("aébc");
        editor.set_cursor_position(0, 3);
        assert_eq!((editor.row(), editor.col()), (0, 2));
    }

    #[test]
    fn cursor_clamps_to_the_buffer() {
        let mut editor = editor_with("abc\ndef");
        editor.set_cursor_position(9, 9);
        assert_eq!((editor.row(), editor.col()), (1, 3));
    }

    #[test]
    fn selection_enters_visual_mode_at_the_end() {
        let mut editor = editor_with("hello");
        editor.set_selection_ends((0, 1), (0, 4));
        assert_eq!(editor.mode(), Mode::Visual);
        assert_eq!(editor.selection_range(), Some(((0, 1), (0, 4))));
        assert_eq!((editor.row(), editor.col()), (0, 4));
    }

    #[test]
    fn clearing_selection_leaves_visual_mode() {
        let mut editor = editor_with("hello");
        editor.set_selection_ends((0, 1), (0, 4));
        editor.clear_visual_selection();
        assert_eq!(editor.mode(), Mode::Normal);
        assert!(editor.selection_range().is_none());
    }

    #[test]
    fn switching_modes_normalizes_state() {
        let mut editor = editor_with("hello");
        assert!(editor.set_editor_mode(Mode::Insert));
        assert_eq!(editor.mode(), Mode::Insert);
        assert!(editor.set_editor_mode(Mode::VisualLine));
        assert_eq!(editor.mode(), Mode::VisualLine);
        assert!(editor.set_editor_mode(Mode::Replace));
        assert_eq!(editor.mode(), Mode::Replace);
        assert!(editor.set_editor_mode(Mode::Normal));
        assert_eq!(editor.mode(), Mode::Normal);
        assert!(editor.selection_range().is_none());
    }

    #[test]
    fn refuses_input_capturing_modes() {
        let mut editor = editor_with("hello");
        assert!(!editor.set_editor_mode(Mode::Command));
        assert!(!editor.set_editor_mode(Mode::Search));
    }
}
