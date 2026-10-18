use super::Editor;

// TODO(join): remove comment leaders when formatoptions contains j.
impl Editor {
    pub(super) fn join_lines(&mut self, without_space: bool) {
        let count = self.take_count().max(2);
        let row = self.cursor.row();
        let last = (row + count - 1).min(self.buffer.line_count().saturating_sub(1));
        if last == row {
            return;
        }
        self.record_boundary();
        let mut text = self.buffer.line_text(row);
        let mut join_col = 0;
        for index in row + 1..=last {
            let line = self.buffer.line_text(index);
            if index == last {
                join_col = text.chars().count();
            }
            if without_space {
                text.push_str(&line);
                continue;
            }
            let piece = line.trim_start_matches([' ', '\t']);
            if needs_join_space(&text, piece) {
                text.push(' ');
            }
            text.push_str(piece);
        }
        self.buffer.replace_lines(row, row, &text);
        self.buffer.delete_lines(row + 1, last);
        let max = self.buffer.line_len(row).saturating_sub(1);
        self.cursor.set_position(row, join_col.min(max));
    }
}

fn needs_join_space(text: &str, piece: &str) -> bool {
    if piece.is_empty() || piece.starts_with(')') || text.is_empty() {
        return false;
    }
    !text.ends_with([' ', '\t'])
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
    fn join_inserts_space_and_removes_indent() {
        let mut e = editor_with("foo\n   bar");
        e.apply(Action::JoinLines);
        assert_eq!(e.text_lines(), vec!["foo bar"]);
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn join_keeps_existing_trailing_space() {
        let mut e = editor_with("foo  \nbar");
        e.apply(Action::JoinLines);
        assert_eq!(e.text_lines(), vec!["foo  bar"]);
    }

    #[test]
    fn join_skips_space_before_closer() {
        let mut e = editor_with("foo\n )bar");
        e.apply(Action::JoinLines);
        assert_eq!(e.text_lines(), vec!["foo)bar"]);
    }

    #[test]
    fn join_without_space_keeps_indent() {
        let mut e = editor_with("foo\n   bar");
        e.apply(Action::JoinLinesNoSpace);
        assert_eq!(e.text_lines(), vec!["foo   bar"]);
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn join_count_joins_multiple_lines() {
        let mut e = editor_with("a\nb\nc\nd");
        e.apply(Action::PushCount(3));
        e.apply(Action::JoinLines);
        assert_eq!(e.text_lines(), vec!["a b c", "d"]);
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn join_at_last_line_does_nothing() {
        let mut e = editor_with("a\nb");
        e.apply(Action::GotoLastLine);
        e.apply(Action::JoinLines);
        assert_eq!(e.text_lines(), vec!["a", "b"]);
    }
}
