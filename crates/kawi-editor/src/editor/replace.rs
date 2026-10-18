use super::Editor;
use crate::editor::tab::TabExpansion;

// TODO(replace): support visual mode r and copying from the adjacent line with ctrl-e and ctrl-y.
impl Editor {
    pub(super) fn replace_char(&mut self, replacement: char) {
        let count = self.take_count();
        let row = self.cursor.row();
        let col = self.cursor.col();
        let width = self.buffer.line_len(row);
        if col >= width || count > width.saturating_sub(col) {
            return;
        }
        if replacement == '\n' {
            self.replace_with_newline(count);
            return;
        }
        if replacement == '\t' {
            if let Some(text) = self.replacement_tabs(count) {
                self.overwrite_chars(row, col, count, &text);
                let indent = self.effective_indent();
                if !indent.expandtab {
                    let end = col.saturating_add(text.chars().count());
                    let max = self.buffer.line_len(row);
                    self.cursor.set_position(row, end.min(max));
                    self.tabify_indent_from(col, indent.tabstop);
                    let cursor = self.cursor.col().saturating_sub(1);
                    self.cursor.set_position(row, cursor);
                }
                return;
            }
        }
        self.overwrite_chars(row, col, count, &replacement.to_string().repeat(count));
    }

    fn replace_with_newline(&mut self, count: usize) {
        let row = self.cursor.row();
        let col = self.cursor.col();
        self.record_boundary();
        self.buffer.delete_range(row, col, row, col + count - 1);
        self.buffer.insert_newline(row, col);
        self.cursor.set_position(row.saturating_add(1), 0);
    }

    fn replacement_tabs(&self, count: usize) -> Option<String> {
        let indent = self.effective_indent();
        let row = self.cursor.row();
        let col = self.cursor.col();
        let mut vcol = self.buffer.screen_col(row, col, indent.tabstop);
        let mut text = String::new();
        for _ in 0..count {
            match self.tab_expansion(row, col, vcol, &indent) {
                TabExpansion::Literal => return None,
                TabExpansion::Spaces(width) => {
                    text.push_str(&" ".repeat(width));
                    vcol = vcol.saturating_add(width);
                }
            }
        }
        Some(text)
    }

    fn overwrite_chars(&mut self, row: usize, col: usize, count: usize, text: &str) {
        self.record_boundary();
        self.buffer.delete_range(row, col, row, col + count - 1);
        for (index, ch) in text.chars().enumerate() {
            self.buffer.insert_char(row, col + index, ch);
        }
        let last = col.saturating_add(text.chars().count().saturating_sub(1));
        let max = self.buffer.line_len(row).saturating_sub(1);
        self.cursor.set_position(row, last.min(max));
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

    #[test]
    fn replace_char_with_count() {
        let mut e = editor_with("abcdef");
        e.apply(Action::JumpTo(0, 1));
        e.apply(Action::PushCount(3));
        e.apply(Action::ReplaceChar('Z'));
        assert_eq!(e.text_lines()[0], "aZZZef");
        assert_eq!((e.row(), e.col()), (0, 3));
        assert!(e.register_text().is_empty());
    }

    #[test]
    fn replace_char_overflow_does_nothing() {
        let mut e = editor_with("abc");
        e.apply(Action::JumpTo(0, 2));
        e.apply(Action::PushCount(2));
        e.apply(Action::ReplaceChar('Z'));
        assert_eq!(e.text_lines()[0], "abc");
    }

    #[test]
    fn replace_with_newline_splits_line() {
        let mut e = editor_with("abcd");
        e.apply(Action::JumpTo(0, 1));
        e.apply(Action::ReplaceChar('\n'));
        assert_eq!(e.text_lines(), vec!["a", "cd"]);
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn replace_with_newline_deletes_count_chars() {
        let mut e = editor_with("abcdef");
        e.apply(Action::JumpTo(0, 1));
        e.apply(Action::PushCount(2));
        e.apply(Action::ReplaceChar('\n'));
        assert_eq!(e.text_lines(), vec!["a", "def"]);
    }

    #[test]
    fn replace_with_tab_uses_smarttab_spaces() {
        let mut e = editor_with("  abc");
        e.set_shiftwidth(4);
        e.apply(Action::JumpTo(0, 1));
        e.apply(Action::ReplaceChar('\t'));
        assert_eq!(e.text_lines()[0], "    abc");
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn replace_with_tab_inserts_literal_tab() {
        let mut e = editor_with("abcdef");
        e.apply(Action::JumpTo(0, 1));
        e.apply(Action::ReplaceChar('\t'));
        assert_eq!(e.text_lines()[0], "a\tcdef");
    }

    #[test]
    fn replace_with_tab_tabifies_inserted_run() {
        let mut e = editor_with("  abc");
        e.set_shiftwidth(4);
        e.apply(Action::JumpTo(0, 1));
        e.apply(Action::PushCount(2));
        e.apply(Action::ReplaceChar('\t'));
        assert_eq!(e.text_lines()[0], " \tbc");
        assert_eq!((e.row(), e.col()), (0, 1));
    }
}
