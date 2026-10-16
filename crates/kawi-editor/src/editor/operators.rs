use super::session::InsertEdit;
use super::Editor;
use crate::actions::{Action, Object};
use crate::find;
use crate::mode::Mode;
use crate::objects;
use crate::register::RegisterKind;
use crate::selection::Selection;
use crate::words;

impl Editor {
    pub(super) fn change_around_word(&mut self) {
        let span = objects::aw_range(&self.buffer, self.cursor.row(), self.cursor.col());
        self.change_object_span(span);
    }

    pub(super) fn change_find(&mut self, target: char, forward: bool, till: bool, repeat: bool) {
        let count = self.take_count();
        let pos = (self.cursor.row(), self.cursor.col());
        if let Some((start, end)) =
            find::find_span(&self.buffer, pos, target, forward, till, count, repeat)
        {
            self.remove_range(start, end);
            self.switch_to_insert();
        }
    }

    pub(super) fn change_inner_word(&mut self) {
        let span = objects::iw_range(&self.buffer, self.cursor.row(), self.cursor.col());
        self.change_object_span(span);
    }

    pub(super) fn change_object(&mut self, inner: bool, object: Object) {
        let count = self.take_count();
        if let Some(span) = self.object_edit(inner, object, count) {
            self.apply_span(span, true);
        }
    }

    pub(super) fn change_line(&mut self) {
        let count = self.take_count();
        let row = self.cursor.row();
        let Some(last) = self.counted_last_row(count) else {
            return;
        };
        self.change_lines(row, last);
    }

    pub(super) fn change_to_start(&mut self) {
        if self.cursor.col() == 0 {
            self.switch_to_insert();
            return;
        }
        let row = self.cursor.row();
        self.record_boundary();
        self.yank_range(row, 0, row, self.cursor.col().saturating_sub(1));
        self.buffer
            .delete_range(row, 0, row, self.cursor.col().saturating_sub(1));
        self.cursor.set_position(row, 0);
        self.switch_to_insert();
    }

    fn change_object_span(&mut self, span: Option<objects::AroundSpan>) {
        let Some(span) = span else {
            self.switch_to_insert();
            return;
        };
        self.remove_object_span(&span);
        self.place_start(span.start);
        self.switch_to_insert();
    }

    fn place_start(&mut self, start: (usize, usize)) {
        let row = start.0.min(self.buffer.max_cursor_row());
        self.cursor.set_position(row, start.1);
    }

    pub(super) fn change_to_end(&mut self) {
        let count = self.take_count();
        let Some(tail) = self.line_tail(count) else {
            self.enter_insert();
            return;
        };
        self.record_boundary();
        self.yank_range(tail.0, tail.1, tail.2, tail.3);
        self.buffer.delete_range(tail.0, tail.1, tail.2, tail.3);
        self.cursor.set_position(tail.0, tail.1);
        self.switch_to_insert();
    }

    pub(super) fn change_word(&mut self) {
        let count = self.take_count();
        let span =
            words::cw_change_range(&self.buffer, self.cursor.row(), self.cursor.col(), count);
        self.apply_span(span, true);
    }

    pub(super) fn change_word_end(&mut self) {
        let count = self.take_count();
        let span = words::ce_range(&self.buffer, self.cursor.row(), self.cursor.col(), count);
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
            (words::WordSpan::Empty { at }, false) => {
                self.cursor.set_position(at.0, at.1);
            }
            (words::WordSpan::Empty { at }, true) => {
                self.cursor.set_position(at.0, at.1);
                self.switch_to_insert();
            }
        }
    }

    pub(super) fn change_lines(&mut self, first: usize, last: usize) {
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

    pub(super) fn remove_lines(&mut self, first: usize, last: usize) {
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

    pub(super) fn collapse_to(&mut self, row: usize, col: usize) {
        self.selection = Some(Selection::new(row, col));
    }

    pub(super) fn delete_around_word(&mut self) {
        let span = objects::aw_range(&self.buffer, self.cursor.row(), self.cursor.col());
        self.delete_object_span(span);
    }

    pub(super) fn delete_backward(&mut self) {
        self.record_maybe();
        self.record_insert(InsertEdit::Backspace);
        let (row, col) = self
            .buffer
            .delete_backward(self.cursor.row(), self.cursor.col());
        self.cursor.set_position(row, col);
    }

    pub(super) fn delete_find(&mut self, target: char, forward: bool, till: bool, repeat: bool) {
        let count = self.take_count();
        let pos = (self.cursor.row(), self.cursor.col());
        if let Some((start, end)) =
            find::find_span(&self.buffer, pos, target, forward, till, count, repeat)
        {
            self.remove_range(start, end);
        }
    }

    pub(super) fn delete_forward(&mut self) {
        if self.mode.is_insert() {
            self.record_maybe();
            self.record_insert(InsertEdit::Delete);
            self.buffer
                .delete_forward(self.cursor.row(), self.cursor.col());
            return;
        }
        let width = self.buffer.line_len(self.cursor.row());
        if width == 0 {
            return;
        }
        let count = self.take_count();
        let end = self
            .cursor
            .col()
            .saturating_add(count.saturating_sub(1))
            .min(width.saturating_sub(1));
        self.record_maybe();
        self.yank_range(self.cursor.row(), self.cursor.col(), self.cursor.row(), end);
        self.buffer
            .delete_range(self.cursor.row(), self.cursor.col(), self.cursor.row(), end);
    }

    // TODO(counts): extend iw and aw spans by count like vim d2iw.
    pub(super) fn delete_inner_word(&mut self) {
        let span = objects::iw_range(&self.buffer, self.cursor.row(), self.cursor.col());
        self.delete_object_span(span);
    }

    pub(super) fn delete_object(&mut self, inner: bool, object: Object) {
        let count = self.take_count();
        if let Some(span) = self.object_edit(inner, object, count) {
            self.apply_span(span, false);
        }
    }

    pub(super) fn object_edit(
        &self,
        inner: bool,
        object: Object,
        count: usize,
    ) -> Option<words::WordSpan> {
        let (row, col) = (self.cursor.row(), self.cursor.col());
        match object {
            Object::Paren => {
                objects::brackets::bracket_range(&self.buffer, row, col, '(', ')', inner, count)
            }
            Object::Brace => {
                objects::brackets::bracket_range(&self.buffer, row, col, '{', '}', inner, count)
            }
            Object::Bracket => {
                objects::brackets::bracket_range(&self.buffer, row, col, '[', ']', inner, count)
            }
            Object::Angle => {
                objects::brackets::bracket_range(&self.buffer, row, col, '<', '>', inner, count)
            }
            Object::SingleQuote => {
                objects::quotes::quote_range(&self.buffer, row, col, '\'', inner, count)
            }
            Object::DoubleQuote => {
                objects::quotes::quote_range(&self.buffer, row, col, '"', inner, count)
            }
            Object::Backtick => {
                objects::quotes::quote_range(&self.buffer, row, col, '`', inner, count)
            }
            Object::Paragraph => {
                objects::paragraph::paragraph_range(&self.buffer, row, !inner, count)
            }
        }
    }

    pub(super) fn delete_line(&mut self) {
        let count = self.take_count();
        let row = self.cursor.row();
        let col = self.cursor.col();
        let Some(last) = self.counted_last_row(count) else {
            return;
        };
        self.register.set_lines(self.take_lines(row, last));
        self.drop_lines(row, last);
        let row = row.min(self.buffer.line_count().saturating_sub(1));
        let max = self.buffer.line_len(row).saturating_sub(1);
        self.cursor.set_position(row, col.min(max));
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
        self.snapshot_visual();
        self.note_visual_repeat(Action::DeleteSelection);
        if self.mode == Mode::VisualLine {
            self.delete_touched_lines();
            return;
        }
        let (start, end) = self.operator_range();
        self.remove_range(start, end);
        self.collapse_to(start.0, start.1);
    }

    pub(super) fn change_selection(&mut self) {
        self.snapshot_visual();
        self.note_visual_repeat(Action::ChangeSelection);
        if self.mode == Mode::VisualLine {
            self.change_touched_lines();
            return;
        }
        let (start, end) = self.operator_range();
        self.remove_range(start, end);
        self.collapse_to(start.0, start.1);
        self.exit_visual();
        self.switch_to_insert();
    }

    fn delete_span_lines(&mut self, span: &objects::AroundSpan) {
        let last = span.end.0.min(self.buffer.line_count().saturating_sub(1));
        if span.start.0 > last {
            return;
        }
        self.register.set_lines(self.take_lines(span.start.0, last));
        self.drop_lines(span.start.0, last);
    }

    fn take_lines(&self, first: usize, last: usize) -> String {
        self.buffer.lines_text(first, last)
    }

    fn drop_lines(&mut self, first: usize, last: usize) {
        self.buffer.delete_lines(first, last);
    }

    pub(super) fn delete_to_end(&mut self) {
        let count = self.take_count();
        let Some(tail) = self.line_tail(count) else {
            return;
        };
        if tail.2 > tail.0 && tail.1 <= self.first_non_blank_of(tail.0) {
            self.delete_line_range(tail.0, tail.2, tail.1);
            return;
        }
        self.record_boundary();
        self.yank_range(tail.0, tail.1, tail.2, tail.3);
        self.buffer.delete_range(tail.0, tail.1, tail.2, tail.3);
        let col = tail.1.min(self.max_col_for_row(tail.0));
        self.cursor.set_position(tail.0, col);
    }

    pub(super) fn delete_to_start(&mut self) {
        let Some((start, end)) = self.line_start_span() else {
            return;
        };
        self.record_boundary();
        self.yank_range(start.0, start.1, end.0, end.1);
        self.buffer.delete_range(start.0, start.1, end.0, end.1);
        self.cursor.set_position(start.0, start.1);
    }

    pub(super) fn line_start_span(&self) -> Option<((usize, usize), (usize, usize))> {
        if self.cursor.col() == 0 {
            return None;
        }
        let row = self.cursor.row();
        Some(((row, 0), (row, self.cursor.col().saturating_sub(1))))
    }

    pub(super) fn delete_word(&mut self) {
        let count = self.take_count();
        let span = words::dw_range(&self.buffer, self.cursor.row(), self.cursor.col(), count);
        self.apply_span(span, false);
    }

    pub(super) fn delete_word_backward(&mut self) {
        let count = self.take_count();
        let span = words::db_range(&self.buffer, self.cursor.row(), self.cursor.col(), count);
        self.apply_span(span, false);
    }

    pub(super) fn delete_word_end(&mut self) {
        let count = self.take_count();
        let span = words::cw_range(&self.buffer, self.cursor.row(), self.cursor.col(), count);
        self.apply_span(span, false);
    }

    pub(super) fn delete_word_end_backward(&mut self) {
        let count = self.take_count();
        let span = words::ge_range(&self.buffer, self.cursor.row(), self.cursor.col(), count);
        self.apply_span(span, false);
    }

    pub(super) fn change_word_end_backward(&mut self) {
        let count = self.take_count();
        let span = words::ge_range(&self.buffer, self.cursor.row(), self.cursor.col(), count);
        self.apply_span(span, true);
    }

    pub(super) fn yank_word_end_backward(&mut self) {
        let count = self.take_count();
        let span = words::ge_range(&self.buffer, self.cursor.row(), self.cursor.col(), count);
        self.yank_word_span(span);
    }

    pub(super) fn first_non_blank_of(&self, row: usize) -> usize {
        self.buffer
            .line(row)
            .map_or(0, |line| kawi_buffer::first_non_blank(&line))
    }

    pub(super) fn insert_char(&mut self, c: char) {
        self.record_maybe();
        self.record_insert(InsertEdit::Char(c));
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
        self.record_insert(InsertEdit::Newline);
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

    pub(super) fn line_tail(&self, count: usize) -> Option<(usize, usize, usize, usize)> {
        if self.line_is_empty_at_cursor() {
            return None;
        }
        let end_row = self.counted_last_row(count)?;
        let last = self.line_width(end_row).saturating_sub(1);
        Some((self.cursor.row(), self.cursor.col(), end_row, last))
    }

    pub(super) fn counted_last_row(&self, count: usize) -> Option<usize> {
        let max = self.buffer.line_count().saturating_sub(1);
        if self.cursor.row() >= max && count > 1 {
            return None;
        }
        Some(
            self.cursor
                .row()
                .saturating_add(count.saturating_sub(1))
                .min(max),
        )
    }

    pub(super) fn open_line_below(&mut self) {
        self.arm_insert_repeat(true);
        self.buffer.begin_change();
        self.open_line_after(self.cursor.row());
        self.switch_to_insert();
    }

    pub(super) fn open_line_after(&mut self, row: usize) {
        let width = self.line_width(row);
        let indent = self.line_indent(row);
        self.insert_newline_at(row, width);
        self.insert_indent(row.saturating_add(1), &indent);
    }

    pub(super) fn operator_range(&self) -> ((usize, usize), (usize, usize)) {
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
        let count = self.take_count();
        self.record_boundary();
        match self.register.kind() {
            RegisterKind::CharWise => self.paste_inline(count),
            RegisterKind::LineWise => self.paste_lines(count),
        }
    }

    pub(super) fn paste_over_selection(&mut self) {
        if self.register.is_empty() {
            return;
        }
        self.snapshot_visual();
        self.note_visual_repeat(Action::PasteOverSelection);
        if self.mode == Mode::VisualLine {
            self.paste_over_lines();
            return;
        }
        let ((start_row, start_col), (end_row, end_col)) = self.operator_range();
        let put_text = self.register.text().to_string();
        let put_kind = self.register.kind();
        self.record_boundary();
        self.yank_range(start_row, start_col, end_row, end_col);
        self.buffer
            .delete_range(start_row, start_col, end_row, end_col);
        self.cursor.set_position(start_row, start_col);
        match put_kind {
            RegisterKind::CharWise => {
                let (_, _) = self.buffer.paste_before(start_row, start_col, &put_text);
                self.cursor.set_position(start_row, start_col);
            }
            RegisterKind::LineWise => self.paste_lines_split(start_row, start_col, &put_text),
        }
    }

    fn paste_over_lines(&mut self) {
        let ((first, _), (last, _)) = self.operator_range();
        let put_text = self.register.text().to_string();
        let put_lines: Vec<String> = put_text.lines().map(str::to_string).collect();
        self.record_boundary();
        self.register.set_lines(self.take_lines(first, last));
        self.buffer.paste_lines_below(last, &put_lines);
        self.drop_lines(first, last);
        self.cursor.set_position(first, 0);
    }

    fn paste_lines_split(&mut self, row: usize, col: usize, text: &str) {
        let lines: Vec<String> = text.lines().map(str::to_string).collect();
        self.buffer.insert_newline(row, col);
        let row = self.buffer.paste_lines_below(row, &lines);
        self.cursor.set_position(row, 0);
    }

    fn paste_inline(&mut self, count: usize) {
        let text = self.register.text().repeat(count.max(1));
        let (row, col) = self
            .buffer
            .paste_after(self.cursor.row(), self.cursor.col(), &text);
        self.cursor.set_position(row, col);
    }

    fn paste_lines(&mut self, count: usize) {
        let yanked: Vec<String> = self.register.text().lines().map(str::to_string).collect();
        let mut lines = Vec::with_capacity(yanked.len().saturating_mul(count.max(1)));
        for _ in 0..count.max(1) {
            lines.extend(yanked.iter().cloned());
        }
        let row = self.buffer.paste_lines_below(self.cursor.row(), &lines);
        self.cursor.set_position(row, self.first_non_blank_of(row));
    }

    fn place_anchor(&mut self, anchor: (usize, usize)) {
        let row = anchor.0.min(self.buffer.line_count().saturating_sub(1));
        self.cursor.set_position(row, anchor.1);
    }

    fn remove_object_span(&mut self, span: &objects::AroundSpan) {
        self.record_boundary_at(span.start.0, span.start.1);
        self.yank_span(span.start.0, span.start.1, span.end.0, span.end.1);
        self.buffer
            .delete_span(span.start.0, span.start.1, span.end.0, span.end.1);
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

    pub(super) fn yank_find(&mut self, target: char, forward: bool, till: bool, repeat: bool) {
        let count = self.take_count();
        let pos = (self.cursor.row(), self.cursor.col());
        if let Some((start, end)) =
            find::find_span(&self.buffer, pos, target, forward, till, count, repeat)
        {
            self.yank_range(start.0, start.1, end.0, end.1);
            self.cursor.set_position(start.0, start.1);
        }
    }

    pub(super) fn yank_inner_word(&mut self) {
        let span = objects::iw_range(&self.buffer, self.cursor.row(), self.cursor.col());
        self.yank_object_span(span);
    }

    pub(super) fn yank_object(&mut self, inner: bool, object: Object) {
        let count = self.take_count();
        let Some(span) = self.object_edit(inner, object, count) else {
            return;
        };
        self.yank_word_span(span);
    }

    fn yank_word_span(&mut self, span: words::WordSpan) {
        match span {
            words::WordSpan::Charwise { start, end } => {
                self.yank_range(start.0, start.1, end.0, end.1);
                self.cursor.set_position(start.0, start.1);
            }
            words::WordSpan::Linewise { first, last } => {
                self.yank_line_range(first, last);
                self.cursor.set_position(first, 0);
            }
            words::WordSpan::Empty { at } => {
                self.cursor.set_position(at.0, at.1);
            }
        }
    }

    pub(super) fn yank_line_range(&mut self, first: usize, last: usize) {
        self.register.set_lines(self.take_lines(first, last));
    }

    pub(super) fn yank_lines(&mut self) {
        let count = self.take_count();
        let row = self.cursor.row();
        let Some(last) = self.counted_last_row(count) else {
            return;
        };
        self.register.set_lines(self.take_lines(row, last));
    }

    fn yank_object_span(&mut self, span: Option<objects::AroundSpan>) {
        if let Some(span) = span {
            self.yank_span(span.start.0, span.start.1, span.end.0, span.end.1);
            self.place_anchor(span.start);
        }
    }

    pub(super) fn yank_range(
        &mut self,
        start_row: usize,
        start_col: usize,
        end_row: usize,
        end_col: usize,
    ) {
        let text = self
            .buffer
            .range_text(start_row, start_col, end_row, end_col);
        self.register.set_text(text);
    }

    pub(super) fn yank_selection(&mut self) {
        self.snapshot_visual();
        if self.mode == Mode::VisualLine {
            self.yank_touched_lines();
            return;
        }
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

    pub(super) fn yank_to_end(&mut self) {
        let count = self.take_count();
        if let Some(tail) = self.line_tail(count) {
            let text = self.buffer.range_text(tail.0, tail.1, tail.2, tail.3);
            self.register.set_text(text);
        }
    }

    pub(super) fn yank_to_start(&mut self) {
        if self.cursor.col() == 0 {
            return;
        }
        let row = self.cursor.row();
        self.yank_range(row, 0, row, self.cursor.col().saturating_sub(1));
        self.cursor.set_desired(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{Action, Object};
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
    fn change_inner_word_on_empty_enters_insert() {
        let mut e = editor_with("\nfoo");
        e.apply(Action::ChangeInnerWord);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!(e.text_lines()[0], "");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn change_inner_word_on_middle_empty_enters_insert() {
        let mut e = editor_with("foo\n\nbar");
        e.apply(Action::MoveDown);
        e.apply(Action::ChangeInnerWord);
        assert_eq!(e.text_lines().len(), 3);
        assert_eq!(e.text_lines()[1], "");
        assert_eq!((e.row(), e.col()), (1, 0));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn change_around_word_on_middle_empty_changes_next() {
        let mut e = editor_with("foo\n\nbar");
        e.apply(Action::MoveDown);
        e.apply(Action::ChangeAroundWord);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!(e.text_lines()[0], "foo");
        assert_eq!(e.text_lines()[1], "");
        assert_eq!((e.row(), e.col()), (1, 0));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn change_inner_word_on_whitespace_clears_run() {
        let mut e = editor_with("foo\n   \nbar");
        e.apply(Action::MoveDown);
        e.apply(Action::MoveRight);
        e.apply(Action::ChangeInnerWord);
        assert_eq!(e.text_lines().len(), 3);
        assert_eq!(e.text_lines()[1], "");
        assert_eq!((e.row(), e.col()), (1, 0));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn change_around_word_on_whitespace_changes_next() {
        let mut e = editor_with("foo\n   \nbar");
        e.apply(Action::MoveDown);
        e.apply(Action::MoveRight);
        e.apply(Action::ChangeAroundWord);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!(e.text_lines()[0], "foo");
        assert_eq!(e.text_lines()[1], "");
        assert_eq!((e.row(), e.col()), (1, 0));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn yank_end_yanks_from_cursor() {
        let mut e = editor_with("abcdef\nghi");
        e.apply(Action::MoveRight);
        e.apply(Action::YankToEnd);
        assert_eq!(e.text_lines()[0], "abcdef");
        assert_eq!((e.row(), e.col()), (0, 1));
        assert_eq!(e.register_kind(), RegisterKind::CharWise);
        e.apply(Action::MoveDown);
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines()[1], "ghbcdefi");
        assert_eq!((e.row(), e.col()), (1, 6));
    }

    #[test]
    fn yank_end_on_empty_is_noop() {
        let mut e = editor_with("\nx");
        e.apply(Action::YankToEnd);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!((e.row(), e.col()), (0, 0));
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!(e.text_lines()[1], "x");
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
        assert_eq!((e.row(), e.col()), (0, 2));
        assert_eq!(e.mode(), Mode::Insert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "abX");
        assert_eq!((e.row(), e.col()), (0, 2));
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
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn delete_inner_paren_deletes_content() {
        let mut e = editor_with("foo (bar) baz");
        for _ in 0..5 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteObject {
            inner: true,
            object: Object::Paren,
        });
        assert_eq!(e.text_lines()[0], "foo () baz");
        assert_eq!((e.row(), e.col()), (0, 5));
    }

    #[test]
    fn delete_around_paren_deletes_brackets() {
        let mut e = editor_with("foo (bar) baz");
        for _ in 0..5 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteObject {
            inner: false,
            object: Object::Paren,
        });
        assert_eq!(e.text_lines()[0], "foo  baz");
        assert_eq!((e.row(), e.col()), (0, 4));
        assert_eq!(e.register_kind(), RegisterKind::CharWise);
    }

    #[test]
    fn delete_inner_empty_paren_moves_to_inner() {
        let mut e = editor_with("foo () bar");
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteObject {
            inner: true,
            object: Object::Paren,
        });
        assert_eq!(e.text_lines()[0], "foo () bar");
        assert_eq!((e.row(), e.col()), (0, 5));
    }

    #[test]
    fn delete_inner_nested_takes_innermost() {
        let mut e = editor_with("foo (a (b) c) bar");
        for _ in 0..8 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteObject {
            inner: true,
            object: Object::Paren,
        });
        assert_eq!(e.text_lines()[0], "foo (a () c) bar");
        assert_eq!((e.row(), e.col()), (0, 8));
    }

    #[test]
    fn delete_inner_unmatched_is_noop() {
        let mut e = editor_with("foo (bar baz");
        for _ in 0..5 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteObject {
            inner: true,
            object: Object::Paren,
        });
        assert_eq!(e.text_lines()[0], "foo (bar baz");
        assert_eq!((e.row(), e.col()), (0, 5));
    }

    #[test]
    fn delete_inner_multiline_removes_middle_line() {
        let mut e = editor_with("foo (\nbar\n) baz");
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteObject {
            inner: true,
            object: Object::Paren,
        });
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!(e.text_lines()[0], "foo (");
        assert_eq!(e.text_lines()[1], ") baz");
        assert_eq!((e.row(), e.col()), (1, 0));
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn delete_around_multiline_is_charwise() {
        let mut e = editor_with("foo (\nbar\n) baz");
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteObject {
            inner: false,
            object: Object::Paren,
        });
        assert_eq!(e.text_lines().len(), 1);
        assert_eq!(e.text_lines()[0], "foo  baz");
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn change_inner_paren_enters_insert() {
        let mut e = editor_with("foo (bar) baz");
        for _ in 0..5 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::ChangeObject {
            inner: true,
            object: Object::Paren,
        });
        assert_eq!(e.text_lines()[0], "foo () baz");
        assert_eq!((e.row(), e.col()), (0, 5));
        assert_eq!(e.mode(), Mode::Insert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "foo (X) baz");
    }

    #[test]
    fn yank_inner_paren_moves_to_start() {
        let mut e = editor_with("foo (bar) baz");
        for _ in 0..6 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::YankObject {
            inner: true,
            object: Object::Paren,
        });
        assert_eq!(e.text_lines()[0], "foo (bar) baz");
        assert_eq!((e.row(), e.col()), (0, 5));
        assert_eq!(e.register_kind(), RegisterKind::CharWise);
    }

    #[test]
    fn yank_around_paren_moves_to_bracket() {
        let mut e = editor_with("foo (bar) baz");
        for _ in 0..5 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::YankObject {
            inner: false,
            object: Object::Paren,
        });
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn delete_inner_brace_and_bracket() {
        let mut e = editor_with("foo {bar} baz");
        for _ in 0..5 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteObject {
            inner: true,
            object: Object::Brace,
        });
        assert_eq!(e.text_lines()[0], "foo {} baz");
        let mut e = editor_with("foo [bar] baz");
        for _ in 0..5 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteObject {
            inner: true,
            object: Object::Bracket,
        });
        assert_eq!(e.text_lines()[0], "foo [] baz");
    }

    #[test]
    fn delete_inner_single_quote() {
        let mut e = editor_with("a 'bc' d");
        for _ in 0..4 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteObject {
            inner: true,
            object: Object::SingleQuote,
        });
        assert_eq!(e.text_lines()[0], "a '' d");
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn delete_around_single_quote_takes_trailing_space() {
        let mut e = editor_with("a 'bc' d");
        for _ in 0..4 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteObject {
            inner: false,
            object: Object::SingleQuote,
        });
        assert_eq!(e.text_lines()[0], "a d");
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn delete_inner_escaped_quote() {
        let mut e = editor_with("a 'b\\'c' d");
        for _ in 0..4 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteObject {
            inner: true,
            object: Object::SingleQuote,
        });
        assert_eq!(e.text_lines()[0], "a '' d");
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn delete_inner_adjacent_quotes_is_noop() {
        let mut e = editor_with("a '' b");
        for _ in 0..2 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteObject {
            inner: true,
            object: Object::SingleQuote,
        });
        assert_eq!(e.text_lines()[0], "a '' b");
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn delete_inner_double_quote_and_backtick() {
        let mut e = editor_with("a \"bc\" d");
        for _ in 0..4 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteObject {
            inner: true,
            object: Object::DoubleQuote,
        });
        assert_eq!(e.text_lines()[0], "a \"\" d");
        let mut e = editor_with("a `bc` d");
        for _ in 0..4 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteObject {
            inner: true,
            object: Object::Backtick,
        });
        assert_eq!(e.text_lines()[0], "a `` d");
    }

    #[test]
    fn delete_inner_paragraph_deletes_line() {
        let mut e = editor_with("a\n\nb\n\nc");
        e.apply(Action::DeleteObject {
            inner: true,
            object: Object::Paragraph,
        });
        assert_eq!(e.text_lines().len(), 4);
        assert_eq!(e.text_lines()[0], "");
        assert_eq!(e.text_lines()[1], "b");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn delete_around_paragraph_takes_blank() {
        let mut e = editor_with("a\n\nb\n\nc");
        e.apply(Action::DeleteObject {
            inner: false,
            object: Object::Paragraph,
        });
        assert_eq!(e.text_lines().len(), 3);
        assert_eq!(e.text_lines()[0], "b");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_middle_paragraph() {
        let mut e = editor_with("a\n\nb\n\nc");
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteObject {
            inner: true,
            object: Object::Paragraph,
        });
        assert_eq!(e.text_lines().len(), 4);
        assert_eq!(e.text_lines()[2], "");
        assert_eq!((e.row(), e.col()), (2, 0));
    }

    #[test]
    fn change_paragraph_enters_insert() {
        let mut e = editor_with("a\n\nb\n\nc");
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::ChangeObject {
            inner: true,
            object: Object::Paragraph,
        });
        assert_eq!(e.text_lines()[2], "");
        assert_eq!((e.row(), e.col()), (2, 0));
        assert_eq!(e.mode(), Mode::Insert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[2], "X");
    }

    #[test]
    fn yank_paragraph_keeps_text() {
        let mut e = editor_with("a\n\nb\n\nc");
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::YankObject {
            inner: true,
            object: Object::Paragraph,
        });
        assert_eq!(e.text_lines()[2], "b");
        assert_eq!((e.row(), e.col()), (2, 0));
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
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
    fn delete_word_end_backward() {
        let mut e = editor_with("foo BAR baz");
        for _ in 0..7 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteWordEndBackward);
        assert_eq!(e.text_lines()[0], "foo BAbaz");
        assert_eq!((e.row(), e.col()), (0, 6));
        assert_eq!(e.register_text(), "R ");
    }

    #[test]
    fn delete_word_end_backward_mid_word() {
        let mut e = editor_with("foo BAR baz");
        for _ in 0..5 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteWordEndBackward);
        assert_eq!(e.text_lines()[0], "foR baz");
        assert_eq!((e.row(), e.col()), (0, 2));
        assert_eq!(e.register_text(), "o BA");
    }

    #[test]
    fn delete_word_end_backward_count() {
        let mut e = editor_with("aa bb cc dd");
        for _ in 0..10 {
            e.apply(Action::MoveRight);
        }
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteWordEndBackward);
        assert_eq!(e.text_lines()[0], "aa b");
        assert_eq!((e.row(), e.col()), (0, 3));
        assert_eq!(e.register_text(), "b cc dd");
    }

    #[test]
    fn delete_word_end_backward_at_start_noop() {
        let mut e = editor_with("foo");
        e.apply(Action::DeleteWordEndBackward);
        assert_eq!(e.text_lines()[0], "foo");
        assert_eq!(e.register_text(), String::new());
    }

    #[test]
    fn change_word_end_backward() {
        let mut e = editor_with("foo BAR baz");
        for _ in 0..7 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::ChangeWordEndBackward);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "foo BAXbaz");
        assert_eq!((e.row(), e.col()), (0, 6));
    }

    #[test]
    fn yank_word_end_backward() {
        let mut e = editor_with("foo BAR baz");
        for _ in 0..7 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::YankWordEndBackward);
        assert_eq!(e.text_lines()[0], "foo BAR baz");
        assert_eq!(e.register_text(), "R ");
        assert_eq!((e.row(), e.col()), (0, 6));
    }

    #[test]
    fn delete_line_preserves_column() {
        let mut e = editor_with("abcdef\ngh");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteLine);
        assert_eq!(e.text_lines(), vec!["gh".to_string()]);
        assert_eq!((e.row(), e.col()), (0, 1));
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
    fn delete_to_end_count_from_line_start_upgrades_linewise() {
        let mut e = editor_with("abcdef\nghijkl");
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteToEnd);
        assert_eq!(e.text_lines(), vec![String::new()]);
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.register_text(), "abcdef\nghijkl");
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn delete_to_end_count_mid_stays_charwise() {
        let mut e = editor_with("abcdef\nghijkl");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteToEnd);
        assert_eq!(e.text_lines(), vec!["abc".to_string()]);
        assert_eq!((e.row(), e.col()), (0, 2));
        assert_eq!(e.register_text(), "def\nghijkl");
        assert_eq!(e.register_kind(), RegisterKind::CharWise);
    }

    #[test]
    fn delete_to_end_count_at_indent_edge_upgrades_linewise() {
        let mut e = editor_with("  abc\ndef");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteToEnd);
        assert_eq!(e.text_lines(), vec![String::new()]);
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn change_to_end_count_from_start_stays_charwise() {
        let mut e = editor_with("abcdef\ngh");
        push_digits(&mut e, &[2]);
        e.apply(Action::ChangeToEnd);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines(), vec!["X".to_string()]);
        assert_eq!(e.register_text(), "abcdef\ngh");
        assert_eq!(e.register_kind(), RegisterKind::CharWise);
    }

    #[test]
    fn yank_to_end_count_from_start_stays_charwise() {
        let mut e = editor_with("abcdef\ngh");
        push_digits(&mut e, &[2]);
        e.apply(Action::YankToEnd);
        assert_eq!(e.text_lines(), vec!["abcdef".to_string(), "gh".to_string()]);
        assert_eq!(e.register_text(), "abcdef\ngh");
        assert_eq!(e.register_kind(), RegisterKind::CharWise);
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
    fn delete_word_with_count_deletes_words() {
        let mut e = editor_with("foo bar baz");
        push_digits(&mut e, &[2]);
        begin_sequence(&mut e);
        e.apply(Action::DeleteWord);
        assert_eq!(e.text_lines()[0], "baz");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_word_multiplies_counts() {
        let mut e = editor_with("foo bar qux baz");
        push_digits(&mut e, &[2]);
        begin_sequence(&mut e);
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteWord);
        assert_eq!(e.text_lines()[0], "");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_word_backward_with_count() {
        let mut e = editor_with("foo bar baz");
        for _ in 0..10 {
            e.apply(Action::MoveRight);
        }
        push_digits(&mut e, &[2]);
        begin_sequence(&mut e);
        e.apply(Action::DeleteWordBackward);
        assert_eq!(e.text_lines()[0], "foo z");
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn delete_word_end_with_count() {
        let mut e = editor_with("foo bar baz");
        push_digits(&mut e, &[2]);
        begin_sequence(&mut e);
        e.apply(Action::DeleteWordEnd);
        assert_eq!(e.text_lines()[0], " baz");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn change_word_with_count() {
        let mut e = editor_with("foo bar baz");
        push_digits(&mut e, &[2]);
        begin_sequence(&mut e);
        e.apply(Action::ChangeWord);
        assert_eq!(e.text_lines()[0], " baz");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn change_word_end_with_count() {
        let mut e = editor_with("foo bar baz");
        push_digits(&mut e, &[2]);
        begin_sequence(&mut e);
        e.apply(Action::ChangeWordEnd);
        assert_eq!(e.text_lines()[0], " baz");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn delete_line_with_count() {
        let mut e = editor_with("a\nb\nc");
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteLine);
        assert_eq!(e.text_lines().len(), 1);
        assert_eq!(e.text_lines()[0], "c");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn delete_line_with_operator_count() {
        let mut e = editor_with("a\nb\nc");
        begin_sequence(&mut e);
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteLine);
        assert_eq!(e.text_lines().len(), 1);
        assert_eq!(e.text_lines()[0], "c");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_line_count_past_end_is_noop() {
        let mut e = editor_with("a\nb\nc");
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        push_digits(&mut e, &[9]);
        e.apply(Action::DeleteLine);
        assert_eq!(e.text_lines().len(), 3);
        assert_eq!((e.row(), e.col()), (2, 0));
    }

    #[test]
    fn delete_line_count_clamps_to_end() {
        let mut e = editor_with("a\nb\nc");
        e.apply(Action::MoveDown);
        push_digits(&mut e, &[3]);
        e.apply(Action::DeleteLine);
        assert_eq!(e.line_count(), 1);
        assert_eq!(e.text_lines()[0], "a");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn change_line_count_past_end_is_noop() {
        let mut e = editor_with("a\nb\nc");
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        push_digits(&mut e, &[9]);
        e.apply(Action::ChangeLine);
        assert_eq!(e.text_lines().len(), 3);
        assert_eq!(e.mode(), Mode::Normal);
    }

    #[test]
    fn yank_lines_count_past_end_is_noop() {
        let mut e = editor_with("a\nb");
        e.apply(Action::MoveDown);
        e.apply(Action::YankLine);
        push_digits(&mut e, &[5]);
        e.apply(Action::YankLine);
        e.apply(Action::MoveToLineStart);
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines().len(), 3);
        assert_eq!(e.text_lines()[1], "b");
        assert_eq!(e.text_lines()[2], "b");
    }

    #[test]
    fn delete_to_end_count_past_end_is_noop() {
        let mut e = editor_with("ab");
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteToEnd);
        assert_eq!(e.text_lines()[0], "ab");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn change_line_with_count() {
        let mut e = editor_with("a\nb\nc");
        push_digits(&mut e, &[2]);
        e.apply(Action::ChangeLine);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!(e.text_lines()[0], "");
        assert_eq!(e.text_lines()[1], "c");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.mode(), Mode::Insert);
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn yank_lines_with_count_pastes_block() {
        let mut e = editor_with("a\nb\nc");
        push_digits(&mut e, &[2]);
        e.apply(Action::YankLine);
        e.apply(Action::MoveDown);
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines().len(), 5);
        assert_eq!(e.text_lines()[2], "a");
        assert_eq!(e.text_lines()[3], "b");
        assert_eq!((e.row(), e.col()), (2, 0));
    }

    #[test]
    fn delete_to_end_with_count() {
        let mut e = editor_with("abcdef\nghijkl");
        e.apply(Action::MoveRight);
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteToEnd);
        assert_eq!(e.text_lines().len(), 1);
        assert_eq!(e.text_lines()[0], "a");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn change_to_end_with_count() {
        let mut e = editor_with("abcdef\nghijkl");
        e.apply(Action::MoveRight);
        push_digits(&mut e, &[2]);
        e.apply(Action::ChangeToEnd);
        assert_eq!(e.text_lines().len(), 1);
        assert_eq!(e.text_lines()[0], "a");
        assert_eq!((e.row(), e.col()), (0, 1));
        assert_eq!(e.mode(), Mode::Insert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "aX");
    }

    #[test]
    fn yank_to_end_with_count() {
        let mut e = editor_with("abcdef\nghijkl");
        e.apply(Action::MoveRight);
        push_digits(&mut e, &[2]);
        e.apply(Action::YankToEnd);
        assert_eq!(e.register_kind(), RegisterKind::CharWise);
        e.apply(Action::MoveDown);
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines().len(), 3);
        assert_eq!(e.text_lines()[1], "ghbcdef");
        assert_eq!(e.text_lines()[2], "ghijklijkl");
    }

    #[test]
    fn delete_to_end_then_down_keeps_column() {
        let mut e = editor_with("abcdef\ngh");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteToEnd);
        e.apply(Action::MoveDown);
        assert_eq!((e.row(), e.col()), (1, 1));
    }

    #[test]
    fn delete_to_end_multiline_then_down_starts_line() {
        let mut e = editor_with("abcdef\nghijkl\nxy");
        e.apply(Action::MoveRight);
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteToEnd);
        e.apply(Action::MoveDown);
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn delete_forward_with_count() {
        let mut e = editor_with("abcdef");
        for _ in 0..4 {
            e.apply(Action::MoveRight);
        }
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteForward);
        assert_eq!(e.text_lines()[0], "abcd");
        assert_eq!((e.row(), e.col()), (0, 3));
        assert_eq!(e.register_kind(), RegisterKind::CharWise);
    }

    #[test]
    fn delete_forward_with_count_clamps() {
        let mut e = editor_with("abcdef");
        for _ in 0..5 {
            e.apply(Action::MoveRight);
        }
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteForward);
        assert_eq!(e.text_lines()[0], "abcde");
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn delete_forward_yanks_single() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteForward);
        assert_eq!(e.text_lines()[0], "acdef");
        e.apply(Action::MoveToLineStart);
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines()[0], "abcdef");
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn delete_find_forward_deletes_through_char() {
        let mut e = editor_with("axbxc");
        e.apply(Action::DeleteFind {
            target: 'x',
            forward: true,
            till: false,
        });
        assert_eq!(e.text_lines()[0], "bxc");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.register_kind(), RegisterKind::CharWise);
    }

    #[test]
    fn delete_find_till_deletes_before_char() {
        let mut e = editor_with("axbxc");
        e.apply(Action::DeleteFind {
            target: 'x',
            forward: true,
            till: true,
        });
        assert_eq!(e.text_lines()[0], "xbxc");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_find_backward_excludes_start() {
        let mut e = editor_with("abxcx");
        for _ in 0..4 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteFind {
            target: 'x',
            forward: false,
            till: false,
        });
        assert_eq!(e.text_lines()[0], "abx");
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn delete_find_backward_till() {
        let mut e = editor_with("abxcx");
        for _ in 0..4 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteFind {
            target: 'x',
            forward: false,
            till: true,
        });
        assert_eq!(e.text_lines()[0], "abxx");
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn delete_find_with_count() {
        let mut e = editor_with("axbxcxd");
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteFind {
            target: 'x',
            forward: true,
            till: false,
        });
        assert_eq!(e.text_lines()[0], "cxd");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_find_multiplies_counts() {
        let mut e = editor_with("axbxcxdxex");
        push_digits(&mut e, &[2]);
        begin_sequence(&mut e);
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteFind {
            target: 'x',
            forward: true,
            till: false,
        });
        assert_eq!(e.text_lines()[0], "ex");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_find_missing_is_noop() {
        let mut e = editor_with("abcdef");
        e.apply(Action::DeleteFind {
            target: 'z',
            forward: true,
            till: false,
        });
        assert_eq!(e.text_lines()[0], "abcdef");
        assert_eq!((e.row(), e.col()), (0, 0));
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines()[0], "abcdef");
    }

    #[test]
    fn change_find_enters_insert() {
        let mut e = editor_with("axbxc");
        e.apply(Action::ChangeFind {
            target: 'x',
            forward: true,
            till: false,
        });
        assert_eq!(e.text_lines()[0], "bxc");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.mode(), Mode::Insert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines()[0], "Xbxc");
    }

    #[test]
    fn change_find_backward_inserts_at_found() {
        let mut e = editor_with("abxcx");
        for _ in 0..4 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::ChangeFind {
            target: 'x',
            forward: false,
            till: false,
        });
        assert_eq!(e.text_lines()[0], "abx");
        assert_eq!((e.row(), e.col()), (0, 2));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn yank_find_yanks_through_char() {
        let mut e = editor_with("axbxc");
        e.apply(Action::YankFind {
            target: 'x',
            forward: true,
            till: false,
        });
        assert_eq!(e.text_lines()[0], "axbxc");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.register_kind(), RegisterKind::CharWise);
        e.apply(Action::MoveToLineEnd);
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines()[0], "axbxcax");
    }

    #[test]
    fn yank_find_backward_moves_to_found() {
        let mut e = editor_with("abxcx");
        for _ in 0..4 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::YankFind {
            target: 'x',
            forward: false,
            till: false,
        });
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn delete_repeat_find_uses_stored_target() {
        let mut e = editor_with("axbxc");
        e.apply(Action::DeleteFind {
            target: 'x',
            forward: true,
            till: false,
        });
        e.apply(Action::DeleteRepeatFind {
            target: 'x',
            forward: true,
            till: false,
        });
        assert_eq!(e.text_lines()[0], "c");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn paste_with_count_repeats_block() {
        let mut e = editor_with("ab");
        e.apply(Action::YankLine);
        push_digits(&mut e, &[2]);
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines().len(), 3);
        assert_eq!(e.text_lines()[1], "ab");
        assert_eq!(e.text_lines()[2], "ab");
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn paste_charwise_with_count_repeats() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::YankToEnd);
        push_digits(&mut e, &[2]);
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines()[0], "abbcdefbcdefcdef");
        assert_eq!((e.row(), e.col()), (0, 11));
    }

    #[test]
    fn delete_to_start_cuts_to_zero() {
        let mut e = editor_with("abcdef");
        for _ in 0..3 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteToStart);
        assert_eq!(e.text_lines()[0], "def");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.register_kind(), RegisterKind::CharWise);
    }

    #[test]
    fn delete_to_start_at_zero_is_noop() {
        let mut e = editor_with("abcdef");
        e.apply(Action::DeleteToStart);
        assert_eq!(e.text_lines()[0], "abcdef");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn change_to_start_cuts_to_zero() {
        let mut e = editor_with("abcdef");
        for _ in 0..3 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::ChangeToStart);
        assert_eq!(e.text_lines()[0], "def");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn yank_to_start_yanks_before_cursor() {
        let mut e = editor_with("abcdef\nghi");
        for _ in 0..3 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::YankToStart);
        assert_eq!(e.register_kind(), RegisterKind::CharWise);
        e.apply(Action::MoveDown);
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines()[1], "gabchi");
        assert_eq!((e.row(), e.col()), (1, 3));
    }

    #[test]
    fn ignored_count_does_not_leak() {
        let mut e = editor_with("foo bar");
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteInnerWord);
        assert_eq!(e.text_lines()[0], " bar");
        e.apply(Action::MoveRight);
        assert_eq!((e.row(), e.col()), (0, 1));
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
    fn yank_inner_word_moves_to_start() {
        let mut e = editor_with("foo bar");
        e.apply(Action::MoveRight);
        e.apply(Action::YankInnerWord);
        assert_eq!(e.text_lines()[0], "foo bar");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn yank_around_word_moves_to_start() {
        let mut e = editor_with("foo\n   \nbar");
        e.apply(Action::MoveDown);
        e.apply(Action::MoveRight);
        e.apply(Action::YankAroundWord);
        assert_eq!(e.text_lines().len(), 3);
        assert_eq!((e.row(), e.col()), (1, 0));
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
