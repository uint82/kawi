use kawi_buffer::tab_width;

use super::Editor;

impl Editor {
    pub(super) fn insert_copy_below(&mut self) {
        self.insert_copied_line_char(false);
    }

    pub(super) fn insert_copy_above(&mut self) {
        self.insert_copied_line_char(true);
    }

    fn insert_copied_line_char(&mut self, above: bool) {
        let Some(ch) = self.adjacent_line_char(above) else {
            return;
        };
        self.insert_char(ch);
    }

    fn adjacent_line_char(&self, above: bool) -> Option<char> {
        let row = self.cursor.row();
        let target_row = if above { row.checked_sub(1)? } else { row + 1 };
        let line = self.buffer.line(target_row)?;
        let tabstop = self.effective_indent().tabstop;
        let screen = self.buffer.screen_col(row, self.cursor.col(), tabstop);
        char_at_screen(&line, screen, tabstop)
    }
}

fn char_at_screen(line: &str, target: usize, tabstop: usize) -> Option<char> {
    let mut chars = line.chars();
    let mut current = chars.next()?;
    let mut screen = 0;
    while screen < target {
        screen += tab_width(current, screen, tabstop);
        if screen > target {
            break;
        }
        current = chars.next()?;
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use crate::mode::Mode;
    use kawi_buffer::Buffer;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    #[test]
    fn copies_char_from_line_below() {
        let mut editor = editor_with("abc\nxyz\n");
        editor.apply(Action::EnterInsert);
        editor.apply(Action::InsertCopyBelow);
        assert_eq!(editor.text_lines()[0], "xabc");
        editor.apply(Action::InsertCopyBelow);
        assert_eq!(editor.text_lines()[0], "xyabc");
    }

    #[test]
    fn copies_char_from_line_above() {
        let mut editor = editor_with("abc\nxyz\n");
        editor.apply(Action::MoveDown);
        editor.apply(Action::MoveRight);
        editor.apply(Action::EnterInsert);
        editor.apply(Action::InsertCopyAbove);
        assert_eq!(editor.text_lines()[1], "xbyz");
    }

    #[test]
    fn no_op_without_adjacent_line() {
        let mut editor = editor_with("abc\n");
        editor.apply(Action::EnterInsert);
        editor.apply(Action::InsertCopyBelow);
        assert_eq!(editor.text_lines()[0], "abc");
        editor.apply(Action::MoveDown);
        editor.apply(Action::InsertCopyAbove);
        assert_eq!(editor.text_lines()[0], "abc");
    }

    #[test]
    fn no_op_when_adjacent_line_is_short() {
        let mut editor = editor_with("abcdef\nxy\n");
        for _ in 0..3 {
            editor.apply(Action::MoveRight);
        }
        editor.apply(Action::EnterInsert);
        editor.apply(Action::InsertCopyBelow);
        assert_eq!(editor.text_lines()[0], "abcdef");
    }

    #[test]
    fn copies_char_at_end_of_line() {
        let mut editor = editor_with("abc\nxyzw\n");
        editor.apply(Action::EnterInsert);
        editor.apply(Action::MoveToLineEnd);
        editor.apply(Action::InsertCopyBelow);
        assert_eq!(editor.text_lines()[0], "abcw");
    }

    #[test]
    fn copies_tab_when_target_screen_column_is_inside_tab() {
        let mut editor = editor_with("abcdefgh\n\tz\n");
        for _ in 0..7 {
            editor.apply(Action::MoveRight);
        }
        editor.apply(Action::EnterInsert);
        editor.apply(Action::InsertCopyBelow);
        assert_eq!(editor.text_lines()[0], "abcdefg\th");
    }

    #[test]
    fn replace_mode_replaces_char() {
        let mut editor = editor_with("abc\nxyz\n");
        editor.apply(Action::EnterReplace);
        editor.apply(Action::MoveRight);
        editor.apply(Action::InsertCopyBelow);
        assert_eq!(editor.text_lines()[0], "ayc");
        assert_eq!(editor.mode(), Mode::Replace);
    }

    #[test]
    fn copied_char_is_one_change() {
        let mut editor = editor_with("abc\nxyz\n");
        editor.apply(Action::EnterInsert);
        editor.apply(Action::InsertCopyBelow);
        editor.apply(Action::ExitInsert);
        editor.apply(Action::Undo);
        assert_eq!(editor.text_lines()[0], "abc");
    }

    #[test]
    fn char_at_screen_matches_nvim_rules() {
        let buffer = Buffer::from_string("abcdef\n\tz\n");
        let line = buffer.line(1).unwrap();
        assert_eq!(super::char_at_screen(&line, 0, 8), Some('\t'));
        assert_eq!(super::char_at_screen(&line, 7, 8), Some('\t'));
        assert_eq!(super::char_at_screen(&line, 8, 8), Some('z'));
        assert_eq!(super::char_at_screen(&line, 9, 8), None);
    }
}
