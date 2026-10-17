use super::session::InsertEdit;
use super::Editor;

impl Editor {
    pub(super) fn insert_tab(&mut self) {
        self.record_insert(InsertEdit::Tab);
        let indent = self.effective_indent();
        let shift = indent.shift_unit();
        let row = self.cursor.row();
        let col = self.cursor.col();
        let in_indent = col <= self.first_non_blank_of(row);
        let smart = in_indent && indent.tabstop != shift;
        if !indent.expandtab && !smart {
            self.record_maybe();
            self.type_text("\t");
            return;
        }
        let vcol = self.buffer.screen_col(row, col, indent.tabstop);
        let width = if smart {
            shift - (vcol % shift)
        } else {
            indent.tabstop - (vcol % indent.tabstop)
        };
        self.record_maybe();
        self.type_text(&" ".repeat(width));
        if !indent.expandtab {
            self.tabify_indent(indent.tabstop);
        }
    }

    fn type_text(&mut self, text: &str) {
        for ch in text.chars() {
            self.buffer
                .insert_char(self.cursor.row(), self.cursor.col(), ch);
            self.cursor.advance_col();
        }
    }

    fn tabify_indent(&mut self, tabstop: usize) {
        let row = self.cursor.row();
        let Some(line) = self.buffer.line(row) else {
            return;
        };
        let chars: Vec<char> = line.chars().collect();
        let cursor_col = self.cursor.col();
        let mut start = cursor_col;
        while start > 0 && matches!(chars.get(start - 1), Some(' ' | '\t')) {
            start -= 1;
        }
        let want_vcol = self.buffer.screen_col(row, cursor_col, tabstop);
        let mut vcol = self.buffer.screen_col(row, start, tabstop);
        let mut index = start;
        let mut prefix = String::new();
        while index < chars.len() && matches!(chars[index], ' ' | '\t') {
            let width = tabstop - (vcol % tabstop);
            if vcol + width > want_vcol {
                break;
            }
            prefix.push('\t');
            index += 1;
            vcol += width;
        }
        while vcol < want_vcol && chars.get(index) == Some(&' ') {
            prefix.push(' ');
            index += 1;
            vcol += 1;
        }
        if vcol > want_vcol {
            prefix.pop();
            index -= 1;
        }
        let head: String = chars[..start].iter().collect();
        let tail: String = chars[cursor_col..].iter().collect();
        self.buffer
            .replace_lines(row, row, &format!("{head}{prefix}{tail}"));
        self.cursor.set_position(row, index);
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

    fn tab(e: &mut Editor) {
        e.apply(Action::InsertTab);
    }

    #[test]
    fn plain_tab_inserts_tab_character() {
        let mut e = editor_with("ab");
        e.apply(Action::EnterInsert);
        tab(&mut e);
        assert_eq!(e.text_lines()[0], "\tab");
        assert_eq!(e.cursor.col(), 1);
    }

    #[test]
    fn plain_tab_mid_line() {
        let mut e = editor_with("abc");
        e.apply(Action::JumpTo(0, 1));
        e.apply(Action::EnterInsert);
        tab(&mut e);
        assert_eq!(e.text_lines()[0], "a\tbc");
    }

    #[test]
    fn expandtab_fills_to_tabstop() {
        let mut e = editor_with("ab");
        e.set_expandtab(true);
        e.apply(Action::EnterInsert);
        tab(&mut e);
        assert_eq!(e.text_lines()[0], "        ab");
        assert_eq!(e.cursor.col(), 8);
    }

    #[test]
    fn expandtab_uses_screen_column() {
        let mut e = editor_with("a\tb");
        e.set_expandtab(true);
        e.apply(Action::JumpTo(0, 2));
        e.apply(Action::EnterInsert);
        tab(&mut e);
        assert_eq!(e.text_lines()[0], "a\t        b");
        assert_eq!(e.cursor.col(), 10);
    }

    #[test]
    fn smarttab_uses_shiftwidth_in_indent() {
        let mut e = editor_with("");
        e.set_shiftwidth(4);
        e.apply(Action::EnterInsert);
        tab(&mut e);
        assert_eq!(e.text_lines()[0], "    ");
        assert_eq!(e.cursor.col(), 4);
    }

    #[test]
    fn smarttab_tabifies_to_shiftwidth() {
        let mut e = editor_with("");
        e.set_shiftwidth(12);
        e.apply(Action::EnterInsert);
        tab(&mut e);
        assert_eq!(e.text_lines()[0], "\t    ");
        assert_eq!(e.cursor.col(), 5);
    }

    #[test]
    fn smarttab_extends_indent_with_spaces() {
        let mut e = editor_with("  ab");
        e.set_shiftwidth(4);
        e.apply(Action::JumpTo(0, 2));
        e.apply(Action::EnterInsert);
        tab(&mut e);
        assert_eq!(e.text_lines()[0], "    ab");
        assert_eq!(e.cursor.col(), 4);
    }

    #[test]
    fn smarttab_keeps_existing_tab() {
        let mut e = editor_with("\tab");
        e.set_shiftwidth(4);
        e.apply(Action::JumpTo(0, 1));
        e.apply(Action::EnterInsert);
        tab(&mut e);
        assert_eq!(e.text_lines()[0], "\t    ab");
        assert_eq!(e.cursor.col(), 5);
    }

    #[test]
    fn tab_outside_indent_ignores_smarttab() {
        let mut e = editor_with("ab");
        e.set_shiftwidth(4);
        e.apply(Action::JumpTo(0, 1));
        e.apply(Action::EnterInsert);
        tab(&mut e);
        assert_eq!(e.text_lines()[0], "a\tb");
    }

    #[test]
    fn smarttab_disabled_when_shiftwidth_equals_tabstop() {
        let mut e = editor_with("");
        e.apply(Action::EnterInsert);
        tab(&mut e);
        assert_eq!(e.text_lines()[0], "\t");
    }

    #[test]
    fn tab_records_for_repeat() {
        let mut e = editor_with("");
        e.apply(Action::EnterInsert);
        tab(&mut e);
        assert_eq!(e.text_lines()[0], "\t");
        e.apply(Action::ExitInsert);
        e.apply(Action::JumpTo(1, 0));
        e.apply(Action::OpenLineBelow);
        e.apply(Action::RepeatLastChange);
        assert_eq!(e.text_lines()[1], "\t");
    }
}
