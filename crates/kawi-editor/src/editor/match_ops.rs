use super::Editor;
use crate::objects::matchit::{match_target, MatchTarget};

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
        if let Some(count) = self.take_percent_count() {
            self.delete_percent_lines(count);
        } else if let Some((start, end)) = self.match_char_span() {
            self.remove_range(start, end);
        }
        self.reveal_jump();
    }

    pub(super) fn yank_match(&mut self) {
        if let Some(count) = self.take_percent_count() {
            self.yank_percent_lines(count);
        } else if let Some((start, end)) = self.match_char_span() {
            self.yank_range(start.0, start.1, end.0, end.1);
            self.cursor.set_position(start.0, start.1);
        }
        self.reveal_jump();
    }

    pub(super) fn change_match(&mut self) {
        if let Some(count) = self.take_percent_count() {
            self.change_percent_lines(count);
        } else if let Some((start, end)) = self.match_char_span() {
            self.remove_range(start, end);
            self.switch_to_insert();
        }
        self.reveal_jump();
    }

    fn take_percent_count(&mut self) -> Option<usize> {
        if self.operator_count.is_none() && !self.has_post_digits() {
            return None;
        }
        Some(self.take_count())
    }

    fn match_char_span(&self) -> Option<((usize, usize), (usize, usize))> {
        let row = self.cursor.row();
        let col = self.cursor.col();
        match match_target(&self.buffer, row, col)? {
            MatchTarget::Bracket { pos } | MatchTarget::Comment { pos } => {
                Some(order_span((row, col), pos))
            }
            MatchTarget::Hash { line, forward } => Some(self.hash_span(row, col, line, forward)),
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

    fn delete_percent_lines(&mut self, count: usize) {
        let Some(target) = self.percent_line(count) else {
            return;
        };
        let col = self.cursor.col();
        let (first, last) = row_span(self.cursor.row(), target);
        self.remove_lines(first, last);
        self.cursor.set_position(self.cursor.row(), col);
    }

    fn yank_percent_lines(&mut self, count: usize) {
        let Some(target) = self.percent_line(count) else {
            return;
        };
        let col = self.cursor.col();
        let (first, last) = row_span(self.cursor.row(), target);
        self.yank_line_range(first, last);
        self.cursor.set_position(first, col);
    }

    fn change_percent_lines(&mut self, count: usize) {
        let Some(target) = self.percent_line(count) else {
            return;
        };
        let (first, last) = row_span(self.cursor.row(), target);
        self.change_lines(first, last);
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

fn row_span(first: usize, second: usize) -> (usize, usize) {
    (first.min(second), first.max(second))
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
    use crate::register::RegisterKind;
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
    fn match_hash_after_directive_word() {
        let mut e = editor_with("#if FOO\nint a;\n#endif");
        e.apply(Action::JumpTo(0, 4));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (2, 0));
    }

    #[test]
    fn match_hash_column_beats_bracket() {
        let mut e = editor_with("  #if (x)\n#endif");
        e.apply(Action::JumpTo(0, 1));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn match_bracket_beats_hash_after_column() {
        let mut e = editor_with("#if (x)\n#endif");
        e.apply(Action::JumpTo(0, 1));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 6));
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
    fn delete_match_unmatched_is_noop() {
        let mut e = editor_with("xx(yy");
        e.apply(Action::JumpTo(0, 2));
        e.apply(Action::DeleteToMatch);
        assert_eq!(e.text_lines()[0], "xx(yy");
        assert_eq!((e.row(), e.col()), (0, 2));
        assert_eq!(e.register_text(), "");
    }

    #[test]
    fn delete_match_empty_line_is_noop() {
        let mut e = editor_with("aaa\n\nccc");
        e.apply(Action::JumpTo(1, 0));
        e.apply(Action::DeleteToMatch);
        assert_eq!(
            e.text_lines(),
            vec!["aaa".to_string(), String::new(), "ccc".to_string()]
        );
        assert_eq!((e.row(), e.col()), (1, 0));
        assert_eq!(e.register_text(), "");
    }

    #[test]
    fn yank_match_unmatched_is_noop() {
        let mut e = editor_with("xx(yy");
        e.apply(Action::JumpTo(0, 2));
        e.apply(Action::YankToMatch);
        assert_eq!(e.register_text(), "");
    }

    #[test]
    fn change_match_unmatched_is_noop() {
        let mut e = editor_with("xx(yy");
        e.apply(Action::JumpTo(0, 2));
        e.apply(Action::ChangeToMatch);
        assert_eq!(e.text_lines()[0], "xx(yy");
        assert_eq!(e.mode(), Mode::Normal);
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
        assert_eq!(e.text_lines()[1], "    line 6");
        assert_eq!(e.line_count(), 96);
        assert_eq!((e.row(), e.col()), (1, 0));
        assert_eq!(
            e.register_text(),
            "    line 2\n    line 3\n    line 4\n    line 5"
        );
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn delete_percent_line_off_column_keeps_column() {
        let text = (1..=100)
            .map(|i| format!("    line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut e = editor_with(&text);
        e.apply(Action::JumpTo(4, 8));
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteToMatch);
        assert_eq!(e.text_lines()[1], "    line 6");
        assert_eq!(e.line_count(), 96);
        assert_eq!((e.row(), e.col()), (1, 8));
        assert_eq!(
            e.register_text(),
            "    line 2\n    line 3\n    line 4\n    line 5"
        );
    }

    #[test]
    fn delete_percent_line_through_empty_line_is_linewise() {
        let mut e = editor_with("def function_1():\n    a = 1\n    b = 2\n\n    c = 3");
        e.apply(Action::JumpTo(0, 4));
        push_digits(&mut e, &[8, 0]);
        e.apply(Action::DeleteToMatch);
        assert_eq!(e.text_lines(), vec!["    c = 3".to_string()]);
        assert_eq!((e.row(), e.col()), (0, 4));
        assert_eq!(
            e.register_text(),
            "def function_1():\n    a = 1\n    b = 2\n"
        );
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn yank_percent_line_through_empty_line_keeps_split() {
        let mut e = editor_with("def function_1():\n    a = 1\n    b = 2\n\n    c = 3");
        e.apply(Action::JumpTo(0, 4));
        push_digits(&mut e, &[8, 0]);
        e.apply(Action::YankToMatch);
        assert_eq!(
            e.register_text(),
            "def function_1():\n    a = 1\n    b = 2\n"
        );
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
        assert_eq!((e.row(), e.col()), (0, 4));
        assert_eq!(e.text_lines().len(), 5);
    }

    #[test]
    fn delete_percent_line_self_deletes_line() {
        let text = (1..=100)
            .map(|i| format!("    line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut e = editor_with(&text);
        e.apply(Action::JumpTo(4, 0));
        push_digits(&mut e, &[5]);
        e.apply(Action::DeleteToMatch);
        assert_eq!(e.line_count(), 99);
        assert_eq!(e.text_lines()[4], "    line 6");
        assert_eq!((e.row(), e.col()), (4, 0));
        assert_eq!(e.register_text(), "    line 5");
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn percent_line_multiplies_counts() {
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
        assert_eq!(e.line_count(), 98);
        assert_eq!(e.text_lines()[3], "    line 6");
        assert_eq!((e.row(), e.col()), (3, 0));
        assert_eq!(e.register_text(), "    line 4\n    line 5");
    }

    #[test]
    fn percent_operator_uses_pre_count() {
        let text = (1..=100)
            .map(|i| format!("    line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut e = editor_with(&text);
        e.apply(Action::JumpTo(10, 0));
        push_digits(&mut e, &[2]);
        begin_sequence(&mut e);
        e.apply(Action::DeleteToMatch);
        assert_eq!(e.line_count(), 90);
        assert_eq!(e.text_lines()[1], "    line 12");
        assert_eq!((e.row(), e.col()), (1, 0));
        assert_eq!(
            e.register_text(),
            "    line 2\n    line 3\n    line 4\n    line 5\n    line 6\n    line 7\n    line 8\n    line 9\n    line 10\n    line 11"
        );
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
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.view_top(), 0);
    }

    #[test]
    fn delete_percent_line_backward_reveals_cursor() {
        let mut e = tall_editor();
        place_top(&mut e, 40);
        e.apply(Action::JumpTo(49, 0));
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteToMatch);
        assert_eq!(e.line_count(), 51);
        assert_eq!(e.text_lines()[1], "    line 51");
        assert_eq!((e.row(), e.col()), (1, 0));
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

    #[test]
    fn match_skips_bracket_inside_string() {
        let mut e = editor_with("x = f(\"a)b\");");
        e.apply(Action::JumpTo(0, 5));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 11));
    }

    #[test]
    fn match_backward_stops_at_opening_quote() {
        let mut e = editor_with("x = f(\"a)b\");");
        e.apply(Action::JumpTo(0, 8));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 8));
    }

    #[test]
    fn match_from_inside_string_matches_inside() {
        let mut e = editor_with("f(\"a(b)c\")");
        e.apply(Action::JumpTo(0, 4));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 6));
    }

    #[test]
    fn match_skips_single_quoted_chars() {
        let mut e = editor_with("a = ['(', ')']");
        e.apply(Action::JumpTo(0, 4));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 13));
    }

    #[test]
    fn match_backward_skips_single_quoted_chars() {
        let mut e = editor_with("a = ['(', ')']");
        e.apply(Action::JumpTo(0, 13));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn match_escaped_quote_keeps_string_open() {
        let mut e = editor_with("f(\"a\\\")b\")");
        e.apply(Action::JumpTo(0, 1));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 9));
    }

    #[test]
    fn match_across_continued_string_line() {
        let mut e = editor_with("x = \"a\\\n(b) c\"");
        e.apply(Action::JumpTo(1, 0));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (1, 2));
    }

    #[test]
    fn match_escaped_brackets_pair_up() {
        let mut e = editor_with("\\(a\\)");
        e.apply(Action::JumpTo(0, 1));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn match_escaped_bracket_skips_unescaped_close() {
        let mut e = editor_with("\\(a)b\\)");
        e.apply(Action::JumpTo(0, 1));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 6));
    }

    #[test]
    fn delete_match_skips_bracket_inside_string() {
        let mut e = editor_with("x = f(\"a)b\");");
        e.apply(Action::JumpTo(0, 5));
        e.apply(Action::DeleteToMatch);
        assert_eq!(e.text_lines()[0], "x = f;");
        assert_eq!(e.register_text(), "(\"a)b\")");
        assert_eq!((e.row(), e.col()), (0, 5));
    }

    #[test]
    fn match_skips_quoted_brackets_in_call() {
        let mut e = editor_with("foo(\"a)b\", 'c', ')');");
        e.apply(Action::JumpTo(0, 3));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 19));
    }

    #[test]
    fn match_multiline_string_odd_quotes_takes_inner_close() {
        let mut e = editor_with("bar(\"multi\nline ) string\", x);");
        e.apply(Action::JumpTo(0, 3));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (1, 5));
    }

    #[test]
    fn match_multiline_string_continuation_skips_inner_close() {
        let mut e = editor_with("bar(\"multi\\\nline ) string\", x);");
        e.apply(Action::JumpTo(0, 3));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (1, 17));
    }

    #[test]
    fn match_escaped_call_pair() {
        let mut e = editor_with("baz(\\(y\\), z);");
        e.apply(Action::JumpTo(0, 3));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 12));
        let mut e = editor_with("baz(\\(y\\), z);");
        e.apply(Action::JumpTo(0, 5));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 8));
    }

    #[test]
    fn match_hash_column_excludes_bracket() {
        let mut e = editor_with("#if (A)\nw(1);\n#endif ");
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (2, 0));
        let mut e = editor_with("#if (A)\nw(1);\n#endif ");
        e.apply(Action::JumpTo(0, 1));
        e.apply(Action::MoveToMatch);
        assert_eq!((e.row(), e.col()), (0, 6));
    }
}
