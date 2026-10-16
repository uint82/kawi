use super::Editor;
use crate::objects::matchit::{match_target, MatchTarget};
use crate::words::dec_pos;

impl Editor {
    pub(super) fn move_to_match(&mut self) {
        self.track_insert_move();
        if self.has_post_digits() {
            let count = self.take_count0();
            if let Some(line) = self.percent_line(count) {
                self.cursor.set_position(line, self.cursor.col());
            }
        } else {
            self.jump_to_match();
        }
        self.reveal_jump();
    }

    fn jump_to_match(&mut self) {
        let row = self.cursor.row();
        let col = self.cursor.col();
        match match_target(&self.buffer, row, col) {
            Some(MatchTarget::Bracket { pos }) | Some(MatchTarget::Comment { pos }) => {
                self.cursor.set_position(pos.0, pos.1);
            }
            Some(MatchTarget::Hash { line, .. }) => {
                self.cursor
                    .set_position(line, self.first_non_blank_of(line));
            }
            None => {}
        }
    }

    pub(super) fn delete_match(&mut self) {
        if self.has_post_digits() {
            self.delete_percent_line();
        } else if self.buffer.line_len(self.cursor.row()) == 0 {
            self.delete_line();
        } else {
            let (start, end) = self.match_char_span();
            self.remove_range(start, end);
        }
        self.reveal_jump();
    }

    pub(super) fn yank_match(&mut self) {
        if self.has_post_digits() {
            self.yank_percent_line();
        } else if self.buffer.line_len(self.cursor.row()) == 0 {
            self.yank_lines();
        } else {
            let (start, end) = self.match_char_span();
            self.yank_range(start.0, start.1, end.0, end.1);
            self.cursor.set_position(start.0, start.1);
        }
        self.reveal_jump();
    }

    pub(super) fn change_match(&mut self) {
        if self.has_post_digits() {
            self.change_percent_line();
        } else if self.buffer.line_len(self.cursor.row()) == 0 {
            let row = self.cursor.row();
            self.change_lines(row, row);
        } else {
            let (start, end) = self.match_char_span();
            self.remove_range(start, end);
            self.switch_to_insert();
        }
        self.reveal_jump();
    }

    fn match_char_span(&self) -> ((usize, usize), (usize, usize)) {
        // TODO(percent): pre-operator counts multiply uniformly instead of
        // nvim operator-pending percent quirks.
        let row = self.cursor.row();
        let col = self.cursor.col();
        match match_target(&self.buffer, row, col) {
            Some(MatchTarget::Bracket { pos }) | Some(MatchTarget::Comment { pos }) => {
                order_span((row, col), pos)
            }
            Some(MatchTarget::Hash { line, forward }) => self.hash_span(row, col, line, forward),
            None => ((row, col), (row, col)),
        }
    }

    fn hash_span(
        &self,
        row: usize,
        col: usize,
        line: usize,
        forward: bool,
    ) -> ((usize, usize), (usize, usize)) {
        if forward {
            let last = self.buffer.line_len(line).saturating_sub(1);
            ((row, col), (line, last))
        } else {
            ((line, 0), (row, col))
        }
    }

    fn delete_percent_line(&mut self) {
        let count = self.take_count0();
        let Some(target) = self.percent_line(count) else {
            return;
        };
        let row = self.cursor.row();
        let col = self.cursor.col();
        let (first, last) = (row.min(target), row.max(target));
        if first == last {
            return;
        }
        if col == 0 {
            return self.remove_lines(first, last.saturating_sub(1));
        }
        self.delete_percent_span(first, last, col);
    }

    fn yank_percent_line(&mut self) {
        let count = self.take_count0();
        let Some(target) = self.percent_line(count) else {
            return;
        };
        let row = self.cursor.row();
        let col = self.cursor.col();
        let (first, last) = (row.min(target), row.max(target));
        if first == last {
            return;
        }
        if col == 0 {
            self.yank_line_range(first, last.saturating_sub(1));
            self.cursor.set_position(first, col);
            return;
        }
        self.yank_percent_span(first, last, col);
    }

    fn change_percent_line(&mut self) {
        let count = self.take_count0();
        let Some(target) = self.percent_line(count) else {
            return;
        };
        let row = self.cursor.row();
        let col = self.cursor.col();
        let (first, last) = (row.min(target), row.max(target));
        if first == last {
            return;
        }
        if col == 0 {
            return self.change_lines(first, last.saturating_sub(1));
        }
        self.delete_percent_span(first, last, col);
        self.switch_to_insert();
    }

    fn delete_percent_span(&mut self, first: usize, last: usize, col: usize) {
        let end = self.percent_span_end(last, col);
        self.remove_range((first, col), end);
    }

    fn yank_percent_span(&mut self, first: usize, last: usize, col: usize) {
        let end = self.percent_span_end(last, col);
        self.yank_range(first, col, end.0, end.1);
        self.cursor.set_position(first, col);
    }

    fn percent_span_end(&self, last: usize, col: usize) -> (usize, usize) {
        if col > self.buffer.line_len(last) {
            let prev = last.saturating_sub(1);
            return (prev, self.buffer.line_len(prev).saturating_sub(1));
        }
        let (end, _) = dec_pos(&self.buffer, last, col);
        end
    }

    fn percent_line(&self, count: usize) -> Option<usize> {
        if count == 0 || count > 100 {
            return None;
        }
        let lines = self.buffer.line_count();
        let line = lines
            .saturating_mul(count)
            .saturating_add(99)
            .saturating_div(100)
            .clamp(1, lines);
        Some(line.saturating_sub(1))
    }
}

fn order_span(first: (usize, usize), second: (usize, usize)) -> ((usize, usize), (usize, usize)) {
    if first <= second {
        (first, second)
    } else {
        (second, first)
    }
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

    fn push_digits(e: &mut Editor, digits: &[u8]) {
        for digit in digits {
            e.apply(Action::PushCount(*digit));
        }
    }

    fn begin_sequence(e: &mut Editor) {
        e.apply(Action::BeginSequence);
    }

    #[test]
    fn match_open_finds_close() {
        let mut e = editor_with("aaa(bbb(ccc)ddd)eee");
        e.apply(Action::JumpTo(0, 3));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 15));
    }

    #[test]
    fn match_close_finds_open() {
        let mut e = editor_with("aaa(bbb(ccc)ddd)eee");
        e.apply(Action::JumpTo(0, 11));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 7));
    }

    #[test]
    fn match_scans_forward_on_line() {
        let mut e = editor_with("foo(bar)baz");
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 7));
    }

    #[test]
    fn match_past_brackets_stays() {
        let mut e = editor_with("foo(bar)baz");
        e.apply(Action::JumpTo(0, 8));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 8));
    }

    #[test]
    fn match_unmatched_stays() {
        let mut e = editor_with("xx(yy");
        e.apply(Action::JumpTo(0, 2));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn match_crosses_lines() {
        let mut e = editor_with("int f(int a,\nint b) {");
        e.apply(Action::JumpTo(0, 5));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (1, 5));
    }

    #[test]
    fn match_comment_pair() {
        let mut e = editor_with("/* hi */");
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 7));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn match_hash_cycles() {
        let mut e = editor_with("#if FOO\nint a;\n#else\nint b;\n#endif");
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (2, 0));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (4, 0));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn match_hash_nests() {
        let mut e = editor_with("#if A\n#if B\nx;\n#else\ny;\n#endif\nz;\n#endif");
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (7, 0));
        e.apply(Action::JumpTo(3, 0));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (5, 0));
    }

    #[test]
    fn match_hash_needs_directive_column() {
        let mut e = editor_with("#if FOO\nint a;\n#endif");
        e.apply(Action::JumpTo(0, 4));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn match_hash_ignores_other_directives() {
        let mut e = editor_with("#include <x>");
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn percent_count_jumps_by_lines() {
        let text = (1..=100)
            .map(|i| format!("    line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut e = editor_with(&text);
        e.apply(Action::JumpTo(0, 8));
        push_digits(&mut e, &[5, 0]);
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (49, 8));
    }

    #[test]
    fn percent_count_over_hundred_is_noop() {
        let mut e = editor_with("a\nb");
        push_digits(&mut e, &[1, 0, 1]);
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_match_removes_to_close() {
        let mut e = editor_with("int f(int a, int b) {");
        e.apply(Action::JumpTo(0, 5));
        e.apply(Action::DeleteToMatch);
        assert_eq!(e.text_lines()[0], "int f {");
        assert_eq!((e.row(), e.col()), (0, 5));
        assert_eq!(e.register_text(), "(int a, int b)");
    }

    #[test]
    fn delete_match_backward_from_close() {
        let mut e = editor_with("aaa(bbb)ccc");
        e.apply(Action::JumpTo(0, 7));
        e.apply(Action::DeleteToMatch);
        assert_eq!(e.text_lines()[0], "aaaccc");
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn delete_match_unmatched_removes_single_char() {
        let mut e = editor_with("xx(yy");
        e.apply(Action::JumpTo(0, 2));
        e.apply(Action::DeleteToMatch);
        assert_eq!(e.text_lines()[0], "xxyy");
        assert_eq!((e.row(), e.col()), (0, 2));
        assert_eq!(e.register_text(), "(");
    }

    #[test]
    fn delete_match_empty_line_deletes_line() {
        let mut e = editor_with("aaa\n\nccc");
        e.apply(Action::JumpTo(1, 0));
        e.apply(Action::DeleteToMatch);
        assert_eq!(e.text_lines(), vec!["aaa".to_string(), "ccc".to_string()]);
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn yank_match_keeps_cursor_at_start() {
        let mut e = editor_with("int f(int a, int b) {");
        e.apply(Action::JumpTo(0, 11));
        e.apply(Action::YankToMatch);
        assert_eq!(e.register_text(), "(int a,");
        assert_eq!((e.row(), e.col()), (0, 5));
        assert_eq!(e.text_lines()[0], "int f(int a, int b) {");
    }

    #[test]
    fn change_match_enters_insert_at_start() {
        let mut e = editor_with("int f(int a, int b) {");
        e.apply(Action::JumpTo(0, 17));
        e.apply(Action::ChangeToMatch);
        assert_eq!(e.text_lines()[0], "int f) {");
        assert_eq!((e.row(), e.col()), (0, 5));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn visual_match_extends_head() {
        let mut e = editor_with("int f(int a, int b) {");
        e.apply(Action::JumpTo(0, 11));
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveToMatch);
        assert_eq!(e.selection_range(), Some(((0, 5), (0, 11))));
        assert_eq!((e.row(), e.col()), (0, 5));
    }

    #[test]
    fn delete_hash_forward_clears_first_line() {
        let mut e = editor_with("#if FOO\nint a;\n#else\nint b;\n#endif");
        e.apply(Action::DeleteToMatch);
        assert_eq!(
            e.text_lines(),
            vec!["".to_string(), "int b;".to_string(), "#endif".to_string()]
        );
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.register_text(), "#if FOO\nint a;\n#else");
    }

    #[test]
    fn delete_hash_backward_removes_fragment() {
        let mut e = editor_with("#if FOO\nint a;\n#else\nint b;\n#endif");
        e.apply(Action::JumpTo(4, 3));
        e.apply(Action::DeleteToMatch);
        assert_eq!(e.text_lines(), vec!["if".to_string()]);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn yank_hash_backward_moves_to_first() {
        let mut e = editor_with("#if FOO\nint a;\n#else\nint b;\n#endif");
        e.apply(Action::JumpTo(4, 3));
        e.apply(Action::YankToMatch);
        assert_eq!(e.register_text(), "#if FOO\nint a;\n#else\nint b;\n#end");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.text_lines().len(), 5);
    }

    #[test]
    fn delete_percent_line_uses_line_span() {
        let text = (1..=100)
            .map(|i| format!("    line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut e = editor_with(&text);
        e.apply(Action::JumpTo(4, 0));
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteToMatch);
        assert_eq!(e.text_lines()[1], "    line 5");
        assert_eq!(e.line_count(), 97);
        assert_eq!((e.row(), e.col()), (1, 4));
        assert_eq!(e.register_text(), "    line 2\n    line 3\n    line 4");
    }

    #[test]
    fn delete_percent_line_charwise_off_column() {
        let text = (1..=100)
            .map(|i| format!("    line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut e = editor_with(&text);
        e.apply(Action::JumpTo(4, 8));
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteToMatch);
        assert_eq!(e.text_lines()[1], "    line 5");
        assert_eq!((e.row(), e.col()), (1, 8));
        assert_eq!(e.register_text(), " 2\n    line 3\n    line 4\n    line");
    }

    #[test]
    fn delete_percent_span_ending_on_empty_line_keeps_split() {
        let mut e = editor_with("def function_1():\n    a = 1\n    b = 2\n\n    c = 3");
        e.apply(Action::JumpTo(0, 4));
        push_digits(&mut e, &[8, 0]);
        e.apply(Action::DeleteToMatch);
        assert_eq!(
            e.text_lines(),
            vec!["def ".to_string(), "".to_string(), "    c = 3".to_string()]
        );
        assert_eq!((e.row(), e.col()), (0, 3));
        assert_eq!(e.register_text(), "function_1():\n    a = 1\n    b = 2");
    }

    #[test]
    fn yank_percent_span_ending_on_empty_line_keeps_split() {
        let mut e = editor_with("def function_1():\n    a = 1\n    b = 2\n\n    c = 3");
        e.apply(Action::JumpTo(0, 4));
        push_digits(&mut e, &[8, 0]);
        e.apply(Action::YankToMatch);
        assert_eq!(e.register_text(), "function_1():\n    a = 1\n    b = 2");
        assert_eq!((e.row(), e.col()), (0, 4));
        assert_eq!(e.text_lines().len(), 5);
    }

    #[test]
    fn delete_percent_line_self_is_noop() {
        let text = (1..=100)
            .map(|i| format!("    line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut e = editor_with(&text);
        e.apply(Action::JumpTo(4, 0));
        push_digits(&mut e, &[5]);
        e.apply(Action::DeleteToMatch);
        assert_eq!(e.line_count(), 100);
        assert_eq!((e.row(), e.col()), (4, 0));
    }

    #[test]
    fn percent_line_uses_post_count_only() {
        let text = (1..=100)
            .map(|i| format!("    line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut e = editor_with(&text);
        e.apply(Action::JumpTo(4, 0));
        push_digits(&mut e, &[2]);
        begin_sequence(&mut e);
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteToMatch);
        assert_eq!(e.text_lines()[1], "    line 5");
        assert_eq!((e.row(), e.col()), (1, 4));
    }

    fn tall_editor() -> Editor {
        let text = (1..=100)
            .map(|i| format!("    line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut e = editor_with(&text);
        e.set_view_size(80, 20);
        e
    }

    fn place_top(e: &mut Editor, top: usize) {
        e.viewport.place_top(top, e.line_count());
    }

    #[test]
    fn percent_jump_far_centers_view() {
        let mut e = tall_editor();
        push_digits(&mut e, &[5, 0]);
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (49, 0));
        assert_eq!(e.view_top(), 40);
    }

    #[test]
    fn percent_jump_odd_height_centers_exactly() {
        let text = (1..=30)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut e = editor_with(&text);
        e.set_view_size(80, 5);
        push_digits(&mut e, &[4, 0]);
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (11, 0));
        assert_eq!(e.view_top(), 9);
    }

    #[test]
    fn percent_jump_near_scrolls_minimally() {
        let mut e = tall_editor();
        place_top(&mut e, 40);
        e.apply(Action::JumpTo(49, 0));
        push_digits(&mut e, &[6, 1]);
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (60, 0));
        assert_eq!(e.view_top(), 41);
    }

    #[test]
    fn percent_jump_visible_keeps_view() {
        let mut e = tall_editor();
        place_top(&mut e, 40);
        e.apply(Action::JumpTo(44, 0));
        push_digits(&mut e, &[4, 5]);
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (44, 0));
        assert_eq!(e.view_top(), 40);
    }

    #[test]
    fn percent_jump_up_far_centers_view() {
        let mut e = tall_editor();
        place_top(&mut e, 40);
        e.apply(Action::JumpTo(49, 0));
        push_digits(&mut e, &[3, 0]);
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (29, 0));
        assert_eq!(e.view_top(), 20);
    }

    #[test]
    fn percent_jump_up_near_scrolls_minimally() {
        let mut e = tall_editor();
        place_top(&mut e, 40);
        e.apply(Action::JumpTo(44, 0));
        push_digits(&mut e, &[3, 6]);
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (35, 0));
        assert_eq!(e.view_top(), 35);
    }

    #[test]
    fn delete_percent_line_reveals_cursor() {
        let mut e = tall_editor();
        e.apply(Action::JumpTo(0, 0));
        push_digits(&mut e, &[5, 0]);
        e.apply(Action::DeleteToMatch);
        assert_eq!((e.row(), e.col()), (0, 4));
        assert_eq!(e.view_top(), 0);
    }

    #[test]
    fn delete_percent_line_backward_reveals_cursor() {
        let mut e = tall_editor();
        place_top(&mut e, 40);
        e.apply(Action::JumpTo(49, 0));
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteToMatch);
        assert_eq!(e.line_count(), 52);
        assert_eq!(e.text_lines()[1], "    line 50");
        assert_eq!((e.row(), e.col()), (1, 4));
        assert_eq!(e.view_top(), 0);
    }

    #[test]
    fn match_far_bracket_falls_back_to_minimal_past_max() {
        let mut text = vec!["(".to_string()];
        text.extend((1..=29).map(|i| format!("x{i}")));
        text.push(")".to_string());
        let mut e = editor_with(&text.join("\n"));
        e.set_view_size(80, 20);
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (30, 0));
        assert_eq!(e.view_top(), 11);
    }

    #[test]
    fn match_mid_bracket_scrolls_past_minimal() {
        let mut text = vec!["(".to_string()];
        text.extend((1..=29).map(|i| format!("x{i}")));
        text.push(")".to_string());
        text.extend((32..=100).map(|i| format!("y{i}")));
        let mut e = editor_with(&text.join("\n"));
        e.set_view_size(80, 20);
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (30, 0));
        assert_eq!(e.view_top(), 20);
    }
}
