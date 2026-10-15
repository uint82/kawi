use super::Editor;
use crate::objects;
use crate::register::RegisterKind;
use crate::selection::Selection;
use crate::words;

impl Editor {
    pub(super) fn change_around_word(&mut self) {
        let span = objects::aw_range(&self.buffer, self.cursor.row(), self.cursor.col());
        self.change_object_span(span);
    }

    pub(super) fn change_inner_word(&mut self) {
        let span = objects::iw_range(&self.buffer, self.cursor.row(), self.cursor.col());
        self.change_object_span(span);
    }

    pub(super) fn change_line(&mut self) {
        let row = self.cursor.row();
        self.change_lines(row, row);
    }

    fn change_object_span(&mut self, span: Option<objects::AroundSpan>) {
        if self.remove_object_span(span) {
            self.switch_to_insert();
        }
    }

    pub(super) fn change_to_end(&mut self) {
        let Some(tail) = self.line_tail() else {
            self.enter_insert();
            return;
        };
        self.record_boundary();
        self.yank_to_end();
        self.buffer.delete_range(tail.0, tail.1, tail.2, tail.3);
        self.clamp_col_normal();
        self.switch_to_insert();
    }

    pub(super) fn change_word(&mut self) {
        let span = words::cw_change_range(&self.buffer, self.cursor.row(), self.cursor.col());
        self.apply_span(span, true);
    }

    pub(super) fn change_word_end(&mut self) {
        let span = words::ce_range(&self.buffer, self.cursor.row(), self.cursor.col());
        self.apply_span(span, true);
    }

    fn apply_span(&mut self, span: words::WordSpan, insert: bool) {
        match (span, insert) {
            (words::WordSpan::Charwise { start, end }, false) => {
                self.remove_range(start, end);
            }
            (words::WordSpan::Charwise { start, end }, true) => {
                self.remove_range(start, end);
                self.switch_to_insert();
            }
            (words::WordSpan::Linewise { first, last }, false) => {
                self.remove_lines(first, last);
            }
            (words::WordSpan::Linewise { first, last }, true) => {
                self.change_lines(first, last);
            }
            (words::WordSpan::Empty, false) => {}
            (words::WordSpan::Empty, true) => self.switch_to_insert(),
        }
    }

    fn change_lines(&mut self, first: usize, last: usize) {
        let last = last.min(self.buffer.line_count().saturating_sub(1));
        let first = first.min(last);
        self.register.set_lines(self.take_lines(first, last));
        self.record_boundary_at(first, 0);
        if last > first {
            self.drop_lines(first.saturating_add(1), last);
        }
        let start = self.buffer.insert_start_col(first);
        if start < self.line_width(first) {
            self.buffer.delete_range(
                first,
                start,
                first,
                self.line_width(first).saturating_sub(1),
            );
        }
        self.cursor.set_position(first, start);
        self.switch_to_insert();
    }

    fn remove_lines(&mut self, first: usize, last: usize) {
        let last = last.min(self.buffer.line_count().saturating_sub(1));
        if first > last {
            return;
        }
        self.record_boundary_at(first, 0);
        self.register.set_lines(self.take_lines(first, last));
        self.drop_lines(first, last);
        let row = first.min(self.buffer.line_count().saturating_sub(1));
        self.cursor.set_position(row, self.first_non_blank_of(row));
    }

    fn clamp_col_normal(&mut self) {
        let width = self.line_width(self.cursor.row());
        self.cursor
            .set_col(self.cursor.col().min(width.saturating_sub(1)));
    }

    pub(super) fn collapse_to(&mut self, row: usize, col: usize) {
        self.selection = Some(Selection::new(row, col));
    }

    pub(super) fn delete_around_word(&mut self) {
        let span = objects::aw_range(&self.buffer, self.cursor.row(), self.cursor.col());
        self.delete_object_span(span);
    }

    pub(super) fn delete_backward(&mut self) {
        self.record_maybe();
        let (row, col) = self
            .buffer
            .delete_backward(self.cursor.row(), self.cursor.col());
        self.cursor.set_position(row, col);
    }

    pub(super) fn delete_forward(&mut self) {
        if !self.mode.is_insert() && self.buffer.line_len(self.cursor.row()) == 0 {
            return;
        }
        self.record_maybe();
        self.buffer
            .delete_forward(self.cursor.row(), self.cursor.col());
    }

    pub(super) fn delete_inner_word(&mut self) {
        let span = objects::iw_range(&self.buffer, self.cursor.row(), self.cursor.col());
        self.delete_object_span(span);
    }

    pub(super) fn delete_line(&mut self) {
        self.yank_current_line();
        self.buffer.delete_line(self.cursor.row());
        self.cursor.set_col(0);
    }

    fn delete_object_span(&mut self, span: Option<objects::AroundSpan>) -> bool {
        let Some(span) = span else {
            return false;
        };
        self.record_boundary_at(span.start.0, span.start.1);
        if span.spans_lines(&self.buffer) {
            self.delete_span_lines(&span);
        } else {
            self.yank_span(span.start.0, span.start.1, span.end.0, span.end.1);
            self.buffer
                .delete_span(span.start.0, span.start.1, span.end.0, span.end.1);
        }
        self.place_anchor(span.anchor);
        true
    }

    pub(super) fn delete_selection(&mut self) {
        let (start, end) = self.operator_range();
        self.remove_range(start, end);
        self.collapse_to(start.0, start.1);
    }

    fn delete_span_lines(&mut self, span: &objects::AroundSpan) {
        let last = span.end.0.min(self.buffer.line_count().saturating_sub(1));
        if span.start.0 > last {
            return;
        }
        self.register.set_lines(self.take_lines(span.start.0, last));
        self.drop_lines(span.start.0, last);
    }

    fn take_lines(&self, first: usize, last: usize) -> Vec<String> {
        (first..=last)
            .map(|row| self.buffer.line_text(row))
            .collect()
    }

    fn drop_lines(&mut self, first: usize, last: usize) {
        let mut row = last;
        loop {
            self.buffer.delete_line(row);
            if row <= first {
                break;
            }
            row = row.saturating_sub(1);
        }
    }

    pub(super) fn delete_to_end(&mut self) {
        let Some(tail) = self.line_tail() else {
            return;
        };
        self.record_boundary();
        self.yank_to_end();
        self.buffer.delete_range(tail.0, tail.1, tail.2, tail.3);
    }

    pub(super) fn delete_word(&mut self) {
        let span = words::dw_range(&self.buffer, self.cursor.row(), self.cursor.col());
        self.apply_span(span, false);
    }

    pub(super) fn delete_word_backward(&mut self) {
        let span = words::db_range(&self.buffer, self.cursor.row(), self.cursor.col());
        self.apply_span(span, false);
    }

    pub(super) fn delete_word_end(&mut self) {
        let span = words::cw_range(&self.buffer, self.cursor.row(), self.cursor.col());
        self.apply_span(span, false);
    }

    fn first_non_blank_of(&self, row: usize) -> usize {
        self.buffer
            .line(row)
            .map_or(0, |line| kawi_buffer::first_non_blank(&line))
    }

    pub(super) fn insert_char(&mut self, c: char) {
        self.record_maybe();
        self.buffer
            .insert_char(self.cursor.row(), self.cursor.col(), c);
        self.cursor.advance_col();
    }

    fn insert_indent(&mut self, row: usize, indent: &str) {
        for (index, ch) in indent.chars().enumerate() {
            self.buffer.insert_char(row, index, ch);
        }
        self.cursor.set_position(row, indent.chars().count());
    }

    pub(super) fn insert_newline(&mut self) {
        let row = self.cursor.row();
        self.insert_newline_at(row, self.cursor.col());
    }

    fn insert_newline_at(&mut self, row: usize, col: usize) {
        self.record_maybe();
        self.buffer.insert_newline(row, col);
        self.cursor.move_to_line_start(row.saturating_add(1));
    }

    fn line_indent(&self, row: usize) -> String {
        match self.buffer.line_cow(row) {
            Some(line) => line.chars().take_while(|ch| ch.is_whitespace()).collect(),
            None => String::new(),
        }
    }

    fn line_is_empty_at_cursor(&self) -> bool {
        let width = self.line_width(self.cursor.row());
        width == 0 || self.cursor.col() >= width
    }

    fn line_tail(&self) -> Option<(usize, usize, usize, usize)> {
        if self.line_is_empty_at_cursor() {
            return None;
        }
        let row = self.cursor.row();
        let last = self.line_width(row).saturating_sub(1);
        Some((row, self.cursor.col(), row, last))
    }

    pub(super) fn open_line_below(&mut self) {
        let row = self.cursor.row();
        let width = self.line_width(row);
        let indent = self.line_indent(row);
        self.buffer.begin_change();
        self.insert_newline_at(row, width);
        self.insert_indent(row.saturating_add(1), &indent);
        self.switch_to_insert();
    }

    fn operator_range(&self) -> ((usize, usize), (usize, usize)) {
        match self.selection_range() {
            Some(range) => range,
            None => (
                (self.cursor.row(), self.cursor.col()),
                (self.cursor.row(), self.cursor.col()),
            ),
        }
    }

    pub(super) fn paste_after(&mut self) {
        if self.register.is_empty() {
            return;
        }
        self.record_boundary();
        match self.register.kind() {
            RegisterKind::CharWise => self.paste_inline(),
            RegisterKind::LineWise => self.paste_lines(),
        }
    }

    fn paste_inline(&mut self) {
        let text = self.register.text().to_string();
        let (row, col) = self
            .buffer
            .paste_after(self.cursor.row(), self.cursor.col(), &text);
        self.cursor.set_position(row, col);
    }

    fn paste_lines(&mut self) {
        let lines: Vec<String> = self.register.text().lines().map(str::to_string).collect();
        let row = self.buffer.paste_lines_below(self.cursor.row(), &lines);
        self.cursor.set_position(row, self.first_non_blank_of(row));
    }

    fn place_anchor(&mut self, anchor: (usize, usize)) {
        let row = anchor.0.min(self.buffer.line_count().saturating_sub(1));
        self.cursor.set_position(row, anchor.1);
    }

    fn remove_object_span(&mut self, span: Option<objects::AroundSpan>) -> bool {
        let Some(span) = span else {
            return false;
        };
        self.record_boundary_at(span.start.0, span.start.1);
        self.yank_span(span.start.0, span.start.1, span.end.0, span.end.1);
        self.buffer
            .delete_span(span.start.0, span.start.1, span.end.0, span.end.1);
        self.place_anchor(span.anchor);
        true
    }

    fn remove_range(&mut self, start: (usize, usize), end: (usize, usize)) {
        self.record_boundary_at(start.0, start.1);
        self.yank_range(start.0, start.1, end.0, end.1);
        self.buffer.delete_range(start.0, start.1, end.0, end.1);
        self.cursor.set_position(start.0, start.1);
    }

    pub(super) fn yank_around_word(&mut self) {
        let span = objects::aw_range(&self.buffer, self.cursor.row(), self.cursor.col());
        self.yank_object_span(span);
    }

    pub(super) fn yank_current_line(&mut self) {
        let text = self.buffer.line_text(self.cursor.row());
        self.register.set_lines(vec![text]);
    }

    pub(super) fn yank_inner_word(&mut self) {
        let span = objects::iw_range(&self.buffer, self.cursor.row(), self.cursor.col());
        self.yank_object_span(span);
    }

    fn yank_object_span(&mut self, span: Option<objects::AroundSpan>) {
        if let Some(span) = span {
            self.yank_span(span.start.0, span.start.1, span.end.0, span.end.1);
        }
    }

    fn yank_range(&mut self, start_row: usize, start_col: usize, end_row: usize, end_col: usize) {
        let text = self
            .buffer
            .range_text(start_row, start_col, end_row, end_col);
        self.register.set_text(text);
    }

    pub(super) fn yank_selection(&mut self) {
        let ((start_row, start_col), (end_row, end_col)) = self.operator_range();
        self.yank_range(start_row, start_col, end_row, end_col);
        self.cursor.set_position(start_row, start_col);
    }

    fn yank_span(&mut self, start_row: usize, start_col: usize, end_row: usize, end_col: usize) {
        let text = self
            .buffer
            .span_text(start_row, start_col, end_row, end_col);
        self.register.set_text(text);
    }

    fn yank_to_end(&mut self) {
        if let Some(tail) = self.line_tail() {
            let text = self.buffer.range_text(tail.0, tail.1, tail.2, tail.3);
            self.register.set_text(text);
        }
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

    #[test]
    fn change_around_word_enters_insert() {
        let mut e = editor_with("foo bar");
        e.apply(Action::ChangeAroundWord);
        assert_eq!(e.text_lines()[0], "bar");
        assert_eq!(e.mode(), Mode::Insert);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn change_around_word_on_empty_enters_insert() {
        let mut e = editor_with("\nfoo");
        e.apply(Action::ChangeAroundWord);
        assert_eq!(e.text_lines().len(), 1);
        assert_eq!(e.text_lines()[0], "");
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn change_inner_word_enters_insert() {
        let mut e = editor_with("foo bar");
        e.apply(Action::ChangeInnerWord);
        assert_eq!(e.text_lines()[0], " bar");
        assert_eq!(e.mode(), Mode::Insert);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn change_inner_word_on_empty_is_noop() {
        let mut e = editor_with("\nfoo");
        e.apply(Action::ChangeInnerWord);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!(e.mode(), Mode::Normal);
    }

    #[test]
    fn change_line_clears_and_groups_typing() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::ChangeLine);
        assert_eq!(e.text_lines()[0], "");
        assert_eq!(e.mode(), Mode::Insert);
        e.apply(Action::InsertChar('x'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "abcdef");
    }

    #[test]
    fn change_line_keeps_indent() {
        let mut e = editor_with("  ab");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::ChangeLine);
        assert_eq!(e.text_lines()[0], "  ");
        assert_eq!((e.row(), e.col()), (0, 2));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn change_to_end_positions_cursor() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::ChangeToEnd);
        assert_eq!(e.text_lines()[0], "ab");
        assert_eq!((e.row(), e.col()), (0, 1));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn change_word_groups_typing() {
        let mut e = editor_with("foo bar");
        e.apply(Action::ChangeWord);
        e.apply(Action::InsertChar('b'));
        e.apply(Action::InsertChar('a'));
        e.apply(Action::InsertChar('z'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "foo bar");
    }

    #[test]
    fn change_word_keeps_trailing_space() {
        let mut e = editor_with("foo bar");
        e.apply(Action::ChangeWord);
        assert_eq!(e.text_lines()[0], " bar");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn change_word_on_end_changes_single_char() {
        let mut e = editor_with("foo bar");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::ChangeWord);
        assert_eq!(e.text_lines()[0], "fo bar");
        assert_eq!((e.row(), e.col()), (0, 2));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn change_word_on_space_acts_like_delete() {
        let mut e = editor_with("foo   bar");
        for _ in 0..4 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::ChangeWord);
        assert_eq!(e.text_lines()[0], "foo bar");
        assert_eq!((e.row(), e.col()), (0, 4));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn delete_around_word_after_edit_jumps_back() {
        let mut e = editor_with("foo\n   \nbar");
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteAroundWord);
        assert_eq!(e.text_lines(), vec!["foo".to_string(), "".to_string()]);
        assert_eq!(e.byte_count(), 4);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_around_word_at_end_takes_leading() {
        let mut e = editor_with("foo bar");
        for _ in 0..4 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteAroundWord);
        assert_eq!(e.text_lines()[0], "foo");
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn delete_around_word_cross_line_takes_newline() {
        let mut e = editor_with("   \nbar\nbaz");
        e.apply(Action::DeleteAroundWord);
        assert_eq!(e.text_lines().len(), 1);
        assert_eq!(e.text_lines()[0], "baz");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_around_word_deletes_space() {
        let mut e = editor_with("foo bar");
        e.apply(Action::DeleteAroundWord);
        assert_eq!(e.text_lines()[0], "bar");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_around_word_empty_restores_origin() {
        let mut e = editor_with("foo\n\nbar");
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteAroundWord);
        assert_eq!(e.text_lines(), vec!["foo".to_string(), "".to_string()]);
        assert_eq!(e.byte_count(), 4);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_around_word_joins_line() {
        let mut e = editor_with("foo   \nbar");
        e.apply(Action::DeleteAroundWord);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!(e.text_lines()[0], "");
        assert_eq!(e.text_lines()[1], "bar");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_around_word_keeps_leading_spaces() {
        let mut e = editor_with("   foo\nbar");
        for _ in 0..3 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteAroundWord);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!(e.text_lines()[0], "   ");
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn delete_around_word_keeps_trailing_word() {
        let mut e = editor_with("xx\nbar\nbaz");
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteAroundWord);
        assert_eq!(e.text_lines().len(), 3);
        assert_eq!(e.text_lines()[1], "");
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn delete_around_word_midrun_keeps_anchor() {
        let mut e = editor_with("xxxx\n   \nbar");
        e.apply(Action::MoveDown);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteAroundWord);
        assert_eq!(e.text_lines(), vec!["xxxx".to_string(), "".to_string()]);
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn delete_around_word_on_empty_takes_next_word() {
        let mut e = editor_with("\nfoo");
        e.apply(Action::DeleteAroundWord);
        assert_eq!(e.text_lines().len(), 1);
        assert_eq!(e.text_lines()[0], "");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_around_word_same_line_skips_newline() {
        let mut e = editor_with("ab\n   foo\nbar");
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteAroundWord);
        assert_eq!(e.text_lines().len(), 3);
        assert_eq!(e.text_lines()[1], "");
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn delete_around_word_stops_at_empty() {
        let mut e = editor_with("foo   \n\nbar");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteAroundWord);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!(e.text_lines()[0], "foo");
        assert_eq!(e.text_lines()[1], "bar");
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn delete_around_word_yanks_linewise_across_lines() {
        let mut e = editor_with("foo\n   \nbar");
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteAroundWord);
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
        let mut e = editor_with("foo bar");
        e.apply(Action::DeleteAroundWord);
        assert_eq!(e.register_kind(), RegisterKind::CharWise);
    }

    #[test]
    fn delete_forward_empty_line_is_noop() {
        let mut e = editor_with("foo\n\nbar");
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteForward);
        assert_eq!(e.text_lines().len(), 3);
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn delete_forward_last_char_steps_back() {
        let mut e = editor_with("ab");
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteForward);
        assert_eq!(e.text_lines()[0], "a");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_forward_removes() {
        let mut e = editor_with("ab");
        e.apply(Action::DeleteForward);
        assert_eq!(e.text_lines()[0], "b");
    }

    #[test]
    fn delete_inner_word_deletes_run() {
        let mut e = editor_with("foo bar");
        e.apply(Action::DeleteInnerWord);
        assert_eq!(e.text_lines()[0], " bar");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_line_removes_row() {
        let mut e = editor_with("a\nb\nc");
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteLine);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!(e.text_lines()[1], "c");
    }

    #[test]
    fn delete_line_yanks_for_paste() {
        let mut e = editor_with("aa\nbb");
        e.apply(Action::DeleteLine);
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines()[0], "bb");
        assert_eq!(e.text_lines()[1], "aa");
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn delete_to_end_empty_is_noop() {
        let mut e = editor_with("");
        e.apply(Action::DeleteToEnd);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "");
    }

    #[test]
    fn delete_to_end_keeps_head() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteToEnd);
        assert_eq!(e.text_lines()[0], "ab");
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn delete_word_backward_across_empty() {
        let mut e = editor_with("foo\n\nbar");
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteWordBackward);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!(e.text_lines()[0], "foo");
        assert_eq!(e.text_lines()[1], "bar");
        assert_eq!((e.row(), e.col()), (1, 0));
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn delete_word_backward_at_start_is_noop() {
        let mut e = editor_with("foo bar");
        e.apply(Action::DeleteWordBackward);
        assert_eq!(e.text_lines()[0], "foo bar");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_word_backward_deletes_back() {
        let mut e = editor_with("foo bar");
        for _ in 0..4 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteWordBackward);
        assert_eq!(e.text_lines()[0], "bar");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_word_backward_joins_lines() {
        let mut e = editor_with("ab\ncd\nef");
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteWordBackward);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!(e.text_lines()[0], "ab");
        assert_eq!(e.text_lines()[1], "ef");
        assert_eq!((e.row(), e.col()), (1, 0));
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn delete_word_backward_pastes_linewise() {
        let mut e = editor_with("foo\nbar");
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteWordBackward);
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!(e.text_lines()[0], "bar");
        assert_eq!(e.text_lines()[1], "foo");
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn delete_word_backward_removes_prev_line() {
        let mut e = editor_with("foo\nbar");
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteWordBackward);
        assert_eq!(e.text_lines().len(), 1);
        assert_eq!(e.text_lines()[0], "bar");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn delete_word_on_empty_removes_line() {
        let mut e = editor_with("foo\n\nbar");
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteWord);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!(e.text_lines()[0], "foo");
        assert_eq!(e.text_lines()[1], "bar");
        assert_eq!((e.row(), e.col()), (1, 0));
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn delete_word_first_of_two_empties() {
        let mut e = editor_with("\n\n");
        e.apply(Action::DeleteWord);
        assert_eq!(e.line_count(), 1);
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn delete_word_on_last_empty_is_noop() {
        let mut e = editor_with("a\n");
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteWord);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn delete_word_end_on_empty_takes_both_lines() {
        let mut e = editor_with("\nbar");
        e.apply(Action::DeleteWordEnd);
        assert_eq!(e.text_lines().len(), 1);
        assert_eq!(e.text_lines()[0], "");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn delete_word_end_crosses_to_next_word() {
        let mut e = editor_with("foo\nbar");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteWordEnd);
        assert_eq!(e.text_lines().len(), 1);
        assert_eq!(e.text_lines()[0], "fo");
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn delete_word_end_on_last_empty_is_noop() {
        let mut e = editor_with("a\n");
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteWordEnd);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn change_word_end_crosses_to_next_word() {
        let mut e = editor_with("foo\nbar");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::ChangeWordEnd);
        assert_eq!(e.text_lines().len(), 1);
        assert_eq!(e.text_lines()[0], "fo");
        assert_eq!((e.row(), e.col()), (0, 2));
        assert_eq!(e.mode(), Mode::Insert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "foX");
    }

    #[test]
    fn change_word_on_empty_stays_in_place() {
        let mut e = editor_with("foo\n\nbar");
        e.apply(Action::MoveDown);
        e.apply(Action::ChangeWord);
        assert_eq!(e.text_lines().len(), 3);
        assert_eq!(e.text_lines()[1], "");
        assert_eq!((e.row(), e.col()), (1, 0));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn change_word_on_last_empty_enters_insert() {
        let mut e = editor_with("a\n");
        e.apply(Action::MoveDown);
        e.apply(Action::ChangeWord);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!((e.row(), e.col()), (1, 0));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn change_word_on_word_end_changes_single() {
        let mut e = editor_with("foo\nbar");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::ChangeWord);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!(e.text_lines()[0], "fo");
        assert_eq!(e.text_lines()[1], "bar");
        assert_eq!((e.row(), e.col()), (0, 2));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn delete_word_deletes_forward() {
        let mut e = editor_with("foo bar");
        e.apply(Action::DeleteWord);
        assert_eq!(e.text_lines()[0], "bar");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_word_end_at_word_end() {
        let mut e = editor_with("foo bar");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteWordEnd);
        assert_eq!(e.text_lines()[0], "fo");
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn delete_word_end_deletes_to_end() {
        let mut e = editor_with("foo bar");
        e.apply(Action::DeleteWordEnd);
        assert_eq!(e.text_lines()[0], " bar");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_word_end_joins_lines() {
        let mut e = editor_with("foo\nbar");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteWordEnd);
        assert_eq!(e.text_lines().len(), 1);
        assert_eq!(e.text_lines()[0], "fo");
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn delete_word_last_word_trims() {
        let mut e = editor_with("foo bar");
        for _ in 0..4 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteWord);
        assert_eq!(e.text_lines()[0], "foo ");
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn delete_word_undoes_together() {
        let mut e = editor_with("foo bar");
        e.apply(Action::DeleteWord);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "foo bar");
    }

    #[test]
    fn insert_char_advances() {
        let mut e = editor_with("ac");
        e.apply(Action::MoveRight);
        e.apply(Action::InsertChar('b'));
        assert_eq!(e.text_lines()[0], "abc");
        assert_eq!(e.col(), 2);
    }

    #[test]
    fn open_line_below_copies_indent() {
        let mut e = editor_with("  ab");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::OpenLineBelow);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!(e.text_lines()[1], "  ");
        assert_eq!((e.row(), e.col()), (1, 2));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn open_line_below_enters_insert() {
        let mut e = editor_with("ab");
        e.apply(Action::OpenLineBelow);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!((e.row(), e.col()), (1, 0));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn paste_after_deleted_text() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteSelection);
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines()[0], "aebcdf");
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn paste_empty_register_keeps_history() {
        let mut e = editor_with("ab");
        e.apply(Action::PasteAfter);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "ab");
    }

    #[test]
    fn yank_around_word_on_empty_is_noop() {
        let mut e = editor_with("\nfoo");
        e.apply(Action::YankAroundWord);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn yank_inner_word_keeps_text() {
        let mut e = editor_with("foo bar");
        e.apply(Action::YankInnerWord);
        assert_eq!(e.text_lines()[0], "foo bar");
        assert_eq!((e.row(), e.col()), (0, 0));
        e.apply(Action::YankAroundWord);
        assert_eq!(e.text_lines()[0], "foo bar");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn yank_line_pastes_below() {
        let mut e = editor_with("aa\nbb\ncc");
        e.apply(Action::MoveDown);
        e.apply(Action::YankLine);
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines().len(), 4);
        assert_eq!(e.text_lines()[2], "bb");
        assert_eq!((e.row(), e.col()), (2, 0));
    }
}
