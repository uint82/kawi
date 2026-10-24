use super::Editor;

impl Editor {
    pub fn set_autoindent(&mut self, autoindent: bool) {
        self.autoindent = autoindent;
    }

    pub fn autoindent(&self) -> bool {
        self.autoindent
    }

    pub(super) fn enter_autoindent(&mut self, row: usize, width: usize) {
        if !self.autoindent {
            return;
        }
        let indent = self.indent_text(width);
        let tail = self
            .buffer
            .line(row)
            .map(|line| line.trim_start().to_string())
            .unwrap_or_default();
        self.set_line_text(row, &format!("{indent}{tail}"));
        self.cursor.set_position(row, indent.chars().count());
        self.pending_autoindent = Some((row, indent));
    }

    pub(super) fn clear_autoindent_line(&mut self) {
        let Some((row, indent)) = self.pending_autoindent.take() else {
            return;
        };
        if self.buffer.line(row).as_deref() != Some(indent.as_str()) {
            return;
        }
        self.set_line_text(row, "");
        if self.cursor.row() == row {
            self.cursor.set_position(row, 0);
        }
    }

    fn set_line_text(&mut self, row: usize, text: &str) {
        let width = self.buffer.line_len(row);
        if width > 0 {
            self.buffer
                .delete_range(row, 0, row, width.saturating_sub(1));
        }
        for (index, ch) in text.chars().enumerate() {
            self.buffer.insert_char(row, index, ch);
        }
    }

    pub(super) fn open_indent(&self, row: usize) -> Option<String> {
        if !self.autoindent {
            return None;
        }
        let tabstop = self.effective_indent().tabstop;
        let width = self.buffer.indent_width(row, tabstop);
        Some(self.indent_text(width))
    }

    pub(super) fn prefix_indent_width(&self, row: usize, col: usize) -> usize {
        let tabstop = self.effective_indent().tabstop;
        let end = self.buffer.indent_end(row).min(col);
        self.buffer.screen_col(row, end, tabstop)
    }

    pub(super) fn backspace_indent(&mut self) -> bool {
        if self.mode != crate::mode::Mode::Insert || !self.autoindent {
            return false;
        }
        let (row, col) = (self.cursor.row(), self.cursor.col());
        if col == 0 {
            return false;
        }
        let Some(line) = self.buffer.line(row) else {
            return false;
        };
        let chars: Vec<char> = line.chars().collect();
        if col > chars.len() || !chars[..col].iter().all(|ch| ch.is_whitespace()) {
            return false;
        }
        let indent = self.effective_indent();
        let step = indent.shift_unit().max(1);
        let width = self.buffer.screen_col(row, col, indent.tabstop);
        if width == 0 {
            return false;
        }
        let want = (width.saturating_sub(1) / step) * step;
        let target = indent_target(&chars[..col], want, indent.tabstop);
        if target >= col {
            return false;
        }
        self.buffer.delete_range(row, target, row, col - 1);
        self.cursor.set_position(row, target);
        true
    }

    pub(super) fn indent_text(&self, width: usize) -> String {
        let indent = self.effective_indent();
        if indent.expandtab {
            return " ".repeat(width);
        }
        let tabstop = indent.tabstop.max(1);
        let mut text = String::new();
        let mut col = 0;
        while col + tabstop <= width {
            text.push('\t');
            col += tabstop;
        }
        text.push_str(&" ".repeat(width.saturating_sub(col)));
        text
    }
}

fn indent_target(chars: &[char], want: usize, tabstop: usize) -> usize {
    let mut vcol = 0;
    for (index, ch) in chars.iter().enumerate() {
        let size = if *ch == '\t' {
            tabstop.max(1) - vcol % tabstop.max(1)
        } else {
            1
        };
        if vcol + size > want {
            return index;
        }
        vcol += size;
    }
    chars.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use kawi_buffer::Buffer;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    #[test]
    fn enter_copies_indent_and_cursor_after_it() {
        let mut e = editor_with("    abc");
        e.apply(Action::AppendLineEnd);
        e.apply(Action::InsertNewline);
        assert_eq!(
            e.text_lines(),
            vec!["    abc".to_string(), "    ".to_string()]
        );
        assert_eq!((e.row(), e.col()), (1, 4));
    }

    #[test]
    fn enter_mid_text_moves_text_and_keeps_indent() {
        let mut e = editor_with("    abc");
        e.apply(Action::JumpTo(0, 5));
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertNewline);
        assert_eq!(
            e.text_lines(),
            vec!["    a".to_string(), "    bc".to_string()]
        );
        assert_eq!((e.row(), e.col()), (1, 4));
    }

    #[test]
    fn enter_inside_indent_does_not_double_indent() {
        let mut e = editor_with("    abc");
        e.apply(Action::JumpTo(0, 2));
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertNewline);
        assert_eq!(e.text_lines(), vec!["  ".to_string(), "  abc".to_string()]);
        assert_eq!((e.row(), e.col()), (1, 2));
    }

    #[test]
    fn enter_at_column_zero_keeps_text_unindented() {
        let mut e = editor_with("    abc");
        e.apply(Action::JumpTo(0, 0));
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertNewline);
        assert_eq!(e.text_lines(), vec!["".to_string(), "abc".to_string()]);
    }

    #[test]
    fn enter_uses_tabs_without_expandtab() {
        let mut e = editor_with("        abc");
        e.apply(Action::AppendLineEnd);
        e.apply(Action::InsertNewline);
        assert_eq!(
            e.text_lines(),
            vec!["        abc".to_string(), "\t".to_string()]
        );
    }

    #[test]
    fn enter_uses_spaces_with_expandtab() {
        let mut e = editor_with("\tabc");
        e.set_expandtab(true);
        e.apply(Action::AppendLineEnd);
        e.apply(Action::InsertNewline);
        assert_eq!(
            e.text_lines(),
            vec!["\tabc".to_string(), "        ".to_string()]
        );
    }

    #[test]
    fn escape_removes_blank_autoindent() {
        let mut e = editor_with("    abc");
        e.apply(Action::AppendLineEnd);
        e.apply(Action::InsertNewline);
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines(), vec!["    abc".to_string(), "".to_string()]);
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn escape_keeps_indent_when_text_typed() {
        let mut e = editor_with("    abc");
        e.apply(Action::AppendLineEnd);
        e.apply(Action::InsertNewline);
        e.apply(Action::InsertChar('x'));
        e.apply(Action::ExitInsert);
        assert_eq!(
            e.text_lines(),
            vec!["    abc".to_string(), "    x".to_string()]
        );
    }

    #[test]
    fn escape_keeps_typed_spaces() {
        let mut e = editor_with("    abc");
        e.apply(Action::AppendLineEnd);
        e.apply(Action::InsertNewline);
        e.apply(Action::InsertChar(' '));
        e.apply(Action::ExitInsert);
        assert_eq!(
            e.text_lines(),
            vec!["    abc".to_string(), "     ".to_string()]
        );
    }

    #[test]
    fn second_enter_clears_blank_autoindent() {
        let mut e = editor_with("    abc");
        e.apply(Action::AppendLineEnd);
        e.apply(Action::InsertNewline);
        e.apply(Action::InsertNewline);
        e.apply(Action::InsertChar('x'));
        e.apply(Action::ExitInsert);
        assert_eq!(
            e.text_lines(),
            vec!["    abc".to_string(), "".to_string(), "    x".to_string()]
        );
    }

    #[test]
    fn backspace_in_autoindent_removes_whole_step() {
        let mut e = editor_with("    abc");
        e.apply(Action::AppendLineEnd);
        e.apply(Action::InsertNewline);
        e.apply(Action::DeleteBackward);
        assert_eq!(e.text_lines(), vec!["    abc".to_string(), "".to_string()]);
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn backspace_in_mid_indent_clears_to_boundary() {
        let mut e = editor_with("        abc");
        e.apply(Action::JumpTo(0, 6));
        e.apply(Action::EnterInsert);
        e.apply(Action::DeleteBackward);
        assert_eq!(e.text_lines(), vec!["  abc".to_string()]);
        assert_eq!(e.col(), 0);
    }

    #[test]
    fn backspace_outside_indent_deletes_one_char() {
        let mut e = editor_with("    abc");
        e.apply(Action::JumpTo(0, 5));
        e.apply(Action::EnterInsert);
        e.apply(Action::DeleteBackward);
        assert_eq!(e.text_lines(), vec!["    bc".to_string()]);
    }

    #[test]
    fn noautoindent_enter_starts_at_column_zero() {
        let mut e = editor_with("    abc");
        e.set_autoindent(false);
        e.apply(Action::AppendLineEnd);
        e.apply(Action::InsertNewline);
        assert_eq!(e.text_lines(), vec!["    abc".to_string(), "".to_string()]);
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn open_below_uses_rebuilt_indent() {
        let mut e = editor_with("\tabc");
        e.apply(Action::OpenLineBelow);
        assert_eq!(e.text_lines(), vec!["\tabc".to_string(), "\t".to_string()]);
        assert_eq!((e.row(), e.col()), (1, 1));
    }

    #[test]
    fn open_below_without_autoindent_has_no_indent() {
        let mut e = editor_with("    abc");
        e.set_autoindent(false);
        e.apply(Action::OpenLineBelow);
        assert_eq!(e.text_lines(), vec!["    abc".to_string(), "".to_string()]);
    }

    #[test]
    fn escape_after_open_removes_indent() {
        let mut e = editor_with("    abc");
        e.apply(Action::OpenLineBelow);
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines(), vec!["    abc".to_string(), "".to_string()]);
    }
}
