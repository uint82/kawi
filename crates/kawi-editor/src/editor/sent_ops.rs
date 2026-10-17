use super::Editor;
use crate::register::RegisterKind;
use crate::sentences::{paragraph_target, sentence_target};

impl Editor {
    pub(super) fn move_to_next_sentence(&mut self) {
        self.track_insert_move();
        let count = self.take_count();
        let row = self.cursor.row();
        let col = self.cursor.col();
        if let Some(found) = sentence_target(&self.buffer, row, col, true, count) {
            self.cursor.set_position(found.0, found.1);
        }
        self.reveal_jump();
    }

    pub(super) fn move_to_prev_sentence(&mut self) {
        self.track_insert_move();
        let count = self.take_count();
        let row = self.cursor.row();
        let col = self.cursor.col();
        if let Some(found) = sentence_target(&self.buffer, row, col, false, count) {
            self.cursor.set_position(found.0, found.1);
        }
        self.reveal_jump();
    }

    pub(super) fn move_to_next_paragraph(&mut self) {
        self.track_insert_move();
        let count = self.take_count();
        let row = self.cursor.row();
        if let Some(found) = paragraph_target(&self.buffer, row, true, count) {
            self.cursor.set_position(found.line, found.col);
        }
        self.reveal_jump();
    }

    pub(super) fn move_to_prev_paragraph(&mut self) {
        self.track_insert_move();
        let count = self.take_count();
        let row = self.cursor.row();
        if let Some(found) = paragraph_target(&self.buffer, row, false, count) {
            self.cursor.set_position(found.line, found.col);
        }
        self.reveal_jump();
    }

    pub(super) fn delete_next_sentence(&mut self) {
        self.delete_statement_span(true, false);
    }

    pub(super) fn delete_prev_sentence(&mut self) {
        self.delete_statement_span(false, false);
    }

    pub(super) fn delete_next_paragraph(&mut self) {
        self.delete_statement_span(true, true);
    }

    pub(super) fn delete_prev_paragraph(&mut self) {
        self.delete_statement_span(false, true);
    }

    pub(super) fn yank_next_sentence(&mut self) {
        self.yank_statement_span(true, false);
    }

    pub(super) fn yank_prev_sentence(&mut self) {
        self.yank_statement_span(false, false);
    }

    pub(super) fn yank_next_paragraph(&mut self) {
        self.yank_statement_span(true, true);
    }

    pub(super) fn yank_prev_paragraph(&mut self) {
        self.yank_statement_span(false, true);
    }

    pub(super) fn change_next_sentence(&mut self) {
        self.change_statement_span(true, false);
    }

    pub(super) fn change_prev_sentence(&mut self) {
        self.change_statement_span(false, false);
    }

    pub(super) fn change_next_paragraph(&mut self) {
        self.change_statement_span(true, true);
    }

    pub(super) fn change_prev_paragraph(&mut self) {
        self.change_statement_span(false, true);
    }

    fn delete_statement_span(&mut self, forward: bool, paragraph: bool) {
        let Some(span) = self.statement_span(forward, paragraph) else {
            return;
        };
        if span.start == span.end && !span.eof {
            return;
        }
        self.record_boundary_at(span.start.0, span.start.1);
        let resolved = self.resolve_span(span, true);
        self.yank_resolved(&resolved);
        match resolved.rows {
            Some((first, last)) => {
                self.drop_lines(first, last);
                let row = first.min(self.buffer.line_count().saturating_sub(1));
                let col = if span.eof {
                    span.start.1
                } else {
                    self.first_non_blank_of(row)
                };
                self.cursor.set_position(row, col);
            }
            None => {
                self.buffer.delete_span(
                    resolved.start.0,
                    resolved.start.1,
                    resolved.edit_end.0,
                    resolved.edit_end.1,
                );
                self.cursor.set_position(span.start.0, span.start.1);
            }
        }
        self.reveal_jump();
    }

    fn yank_statement_span(&mut self, forward: bool, paragraph: bool) {
        let Some(span) = self.statement_span(forward, paragraph) else {
            return;
        };
        if span.start == span.end && !span.eof {
            return;
        }
        let resolved = self.resolve_span(span, false);
        self.yank_resolved(&resolved);
        self.cursor.set_position(span.start.0, span.start.1);
        self.reveal_jump();
    }

    fn change_statement_span(&mut self, forward: bool, paragraph: bool) {
        let Some(span) = self.statement_span(forward, paragraph) else {
            return;
        };
        if span.start == span.end && !span.eof {
            self.switch_to_insert();
            self.reveal_jump();
            return;
        }
        self.record_boundary_at(span.start.0, span.start.1);
        let resolved = self.resolve_span(span, false);
        match resolved.rows {
            Some((first, last)) => self.change_lines(first, last),
            None => {
                self.yank_resolved(&resolved);
                self.buffer.delete_span(
                    resolved.start.0,
                    resolved.start.1,
                    resolved.edit_end.0,
                    resolved.edit_end.1,
                );
                self.cursor.set_position(span.start.0, span.start.1);
                self.switch_to_insert();
            }
        }
        self.reveal_jump();
    }

    fn statement_span(&mut self, forward: bool, paragraph: bool) -> Option<Span> {
        let count = self.take_count();
        let row = self.cursor.row();
        let col = self.cursor.col();
        let (target, eof) = if paragraph {
            let found = paragraph_target(&self.buffer, row, forward, count)?;
            ((found.line, found.col), found.eof)
        } else {
            (
                sentence_target(&self.buffer, row, col, forward, count)?,
                false,
            )
        };
        let (start, end) = if target <= (row, col) {
            (target, (row, col))
        } else {
            ((row, col), target)
        };
        Some(Span { start, end, eof })
    }

    fn resolve_span(&self, span: Span, is_delete: bool) -> ResolvedSpan {
        if span.eof {
            return self.resolve_eof_span(span, is_delete);
        }
        if span.end.0 > span.start.0 && span.end.1 == 0 {
            if self.in_indent(span.start) {
                return ResolvedSpan {
                    start: span.start,
                    yank_start: (span.start.0, 0),
                    yank_end: span.end,
                    edit_end: span.end,
                    kind: RegisterKind::LineWise,
                    rows: Some((span.start.0, span.end.0.saturating_sub(1))),
                };
            }
            let row = span.end.0.saturating_sub(1);
            let backed = (row, self.buffer.line_len(row));
            return ResolvedSpan {
                start: span.start,
                yank_start: span.start,
                yank_end: backed,
                edit_end: backed,
                kind: RegisterKind::CharWise,
                rows: None,
            };
        }
        ResolvedSpan {
            start: span.start,
            yank_start: span.start,
            yank_end: span.end,
            edit_end: span.end,
            kind: RegisterKind::CharWise,
            rows: None,
        }
    }

    fn resolve_eof_span(&self, span: Span, is_delete: bool) -> ResolvedSpan {
        let row = span.end.0;
        let content_end = (row, self.buffer.line_len(row));
        if is_delete && span.start.0 < row && self.in_indent(span.start) {
            return ResolvedSpan {
                start: span.start,
                yank_start: (span.start.0, 0),
                yank_end: content_end,
                edit_end: span.end,
                kind: RegisterKind::LineWise,
                rows: Some((span.start.0, row)),
            };
        }
        ResolvedSpan {
            start: span.start,
            yank_start: span.start,
            yank_end: content_end,
            edit_end: content_end,
            kind: RegisterKind::CharWise,
            rows: None,
        }
    }

    fn in_indent(&self, pos: (usize, usize)) -> bool {
        pos.1 <= self.first_non_blank_of(pos.0)
    }

    fn yank_resolved(&mut self, resolved: &ResolvedSpan) {
        let text = self.buffer.span_text(
            resolved.yank_start.0,
            resolved.yank_start.1,
            resolved.yank_end.0,
            resolved.yank_end.1,
        );
        match resolved.kind {
            RegisterKind::LineWise => self.register.set_lines(text),
            RegisterKind::CharWise => self.register.set_text(text),
        }
    }
}

#[derive(Clone, Copy)]
struct Span {
    start: (usize, usize),
    end: (usize, usize),
    eof: bool,
}

struct ResolvedSpan {
    start: (usize, usize),
    yank_start: (usize, usize),
    yank_end: (usize, usize),
    edit_end: (usize, usize),
    kind: RegisterKind,
    rows: Option<(usize, usize)>,
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

    fn sent_text() -> String {
        "First sentence. Second here.\n\n  Indented start!  Spaced?  End.\nNo terminator here\n(Paren.) \"Quoted.\" Atas.\nEnd!  ".to_string()
    }

    fn macro_text() -> String {
        "intro line\n.PP\npara two\n.IP tag\npara three\n.SH NAME\ntail".to_string()
    }

    fn macro_tail_text() -> String {
        "one\n.PPx rest\ntwo\n.P\nthree\n.p\nfour\n.PP".to_string()
    }

    fn para_text() -> String {
        "para one line1\npara one line2\n\npara two\n\n\npara three line1\npara three line2"
            .to_string()
    }

    #[test]
    fn next_sentence_skips_closer() {
        let mut e = editor_with(&sent_text());
        e.apply(Action::MoveToNextSentence);
        assert_eq!((e.row(), e.col()), (0, 16));
    }

    #[test]
    fn next_sentence_from_inside() {
        let mut e = editor_with(&sent_text());
        e.apply(Action::JumpTo(0, 16));
        e.apply(Action::MoveToNextSentence);
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn next_sentence_indented() {
        let mut e = editor_with(&sent_text());
        e.apply(Action::JumpTo(2, 2));
        e.apply(Action::MoveToNextSentence);
        assert_eq!((e.row(), e.col()), (2, 19));
    }

    #[test]
    fn next_sentence_double_space() {
        let mut e = editor_with(&sent_text());
        e.apply(Action::JumpTo(2, 19));
        e.apply(Action::MoveToNextSentence);
        assert_eq!((e.row(), e.col()), (2, 28));
    }

    #[test]
    fn next_sentence_across_plain_line() {
        let mut e = editor_with(&sent_text());
        e.apply(Action::JumpTo(3, 4));
        e.apply(Action::MoveToNextSentence);
        assert_eq!((e.row(), e.col()), (4, 9));
    }

    #[test]
    fn next_sentence_at_end_stays() {
        let mut e = editor_with(&sent_text());
        e.apply(Action::JumpTo(5, 0));
        e.apply(Action::MoveToNextSentence);
        assert_eq!((e.row(), e.col()), (5, 5));
    }

    #[test]
    fn prev_sentence_to_start() {
        let mut e = editor_with(&sent_text());
        e.apply(Action::JumpTo(0, 16));
        e.apply(Action::MoveToPrevSentence);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn prev_sentence_at_start_stays() {
        let mut e = editor_with(&sent_text());
        e.apply(Action::MoveToPrevSentence);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn prev_sentence_from_blank() {
        let mut e = editor_with(&sent_text());
        e.apply(Action::JumpTo(1, 0));
        e.apply(Action::MoveToPrevSentence);
        assert_eq!((e.row(), e.col()), (0, 16));
    }

    #[test]
    fn next_sentence_from_blank() {
        let mut e = editor_with(&sent_text());
        e.apply(Action::JumpTo(1, 0));
        e.apply(Action::MoveToNextSentence);
        assert_eq!((e.row(), e.col()), (2, 2));
    }

    #[test]
    fn next_sentence_count() {
        let mut e = editor_with(&sent_text());
        push_digits(&mut e, &[2]);
        e.apply(Action::MoveToNextSentence);
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn prev_sentence_from_middle() {
        let mut e = editor_with(&sent_text());
        e.apply(Action::JumpTo(0, 17));
        e.apply(Action::MoveToPrevSentence);
        assert_eq!((e.row(), e.col()), (0, 16));
    }

    #[test]
    fn next_paragraph_to_blank() {
        let mut e = editor_with(&para_text());
        e.apply(Action::MoveToNextParagraph);
        assert_eq!((e.row(), e.col()), (2, 0));
    }

    #[test]
    fn next_paragraph_from_blank() {
        let mut e = editor_with(&para_text());
        e.apply(Action::JumpTo(2, 0));
        e.apply(Action::MoveToNextParagraph);
        assert_eq!((e.row(), e.col()), (4, 0));
    }

    #[test]
    fn prev_paragraph_to_blank() {
        let mut e = editor_with(&para_text());
        e.apply(Action::JumpTo(3, 0));
        e.apply(Action::MoveToPrevParagraph);
        assert_eq!((e.row(), e.col()), (2, 0));
    }

    #[test]
    fn prev_paragraph_at_top_stays() {
        let mut e = editor_with(&para_text());
        e.apply(Action::JumpTo(2, 0));
        e.apply(Action::MoveToPrevParagraph);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn next_paragraph_blank_run_to_last_char() {
        let mut e = editor_with(&para_text());
        e.apply(Action::JumpTo(4, 0));
        e.apply(Action::MoveToNextParagraph);
        assert_eq!((e.row(), e.col()), (7, 15));
    }

    #[test]
    fn next_paragraph_on_last_line_to_last_char() {
        let mut e = editor_with(&para_text());
        e.apply(Action::JumpTo(7, 0));
        e.apply(Action::MoveToNextParagraph);
        assert_eq!((e.row(), e.col()), (7, 15));
    }

    #[test]
    fn next_paragraph_count() {
        let mut e = editor_with(&para_text());
        push_digits(&mut e, &[2]);
        e.apply(Action::MoveToNextParagraph);
        assert_eq!((e.row(), e.col()), (4, 0));
    }

    #[test]
    fn whitespace_line_is_not_boundary() {
        let mut e = editor_with("aaa\n   \nbbb");
        e.apply(Action::MoveToNextParagraph);
        assert_eq!((e.row(), e.col()), (2, 2));
    }

    #[test]
    fn prev_paragraph_top_bound() {
        let mut e = editor_with("x\n\nb\ny\nc");
        e.apply(Action::MoveToPrevParagraph);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn next_paragraph_no_blanks_to_end() {
        let mut e = editor_with("one\ntwo\nthree");
        e.apply(Action::MoveToNextParagraph);
        assert_eq!((e.row(), e.col()), (2, 4));
    }

    #[test]
    fn delete_undo_repeat_sentence() {
        let mut e = editor_with(&sent_text());
        e.apply(Action::JumpTo(0, 13));
        e.apply(Action::DeleteToNextSentence);
        assert_eq!(e.text_lines()[0], "First sentencSecond here.");
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "First sentence. Second here.");
        e.apply(Action::RepeatLastChange);
        assert_eq!(e.text_lines()[0], "First sentencSecond here.");
        assert_eq!((e.row(), e.col()), (0, 13));
        e.apply(Action::RepeatLastChange);
        assert_eq!(e.text_lines()[0], "First sentenc");
        assert_eq!(e.text_lines()[1], "");
    }

    #[test]
    fn delete_next_sentence() {
        let mut e = editor_with(&sent_text());
        e.apply(Action::DeleteToNextSentence);
        assert_eq!(e.text_lines()[0], "Second here.");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.register_text(), "First sentence. ");
        assert_eq!(e.register_kind(), RegisterKind::CharWise);
    }

    #[test]
    fn delete_prev_sentence() {
        let mut e = editor_with(&sent_text());
        e.apply(Action::JumpTo(0, 16));
        e.apply(Action::DeleteToPrevSentence);
        assert_eq!(e.text_lines()[0], "Second here.");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_sentence_mid() {
        let mut e = editor_with(&sent_text());
        e.apply(Action::JumpTo(2, 2));
        e.apply(Action::DeleteToNextSentence);
        assert_eq!(e.text_lines()[2], "  Spaced?  End.");
        assert_eq!((e.row(), e.col()), (2, 2));
        assert_eq!(e.register_text(), "Indented start!  ");
    }

    #[test]
    fn delete_sentence_at_bof_noop() {
        let mut e = editor_with(&sent_text());
        e.apply(Action::DeleteToPrevSentence);
        assert_eq!(e.text_lines()[0], "First sentence. Second here.");
        assert_eq!(e.register_text(), "");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_sentence_at_end_clears_line() {
        let mut e = editor_with(&sent_text());
        e.apply(Action::JumpTo(5, 0));
        e.apply(Action::DeleteToNextSentence);
        assert_eq!(e.text_lines()[5], "");
        assert_eq!((e.row(), e.col()), (5, 0));
        assert_eq!(e.register_text(), "End!  ");
    }

    #[test]
    fn delete_sentence_count_linewise() {
        let mut e = editor_with(&sent_text());
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteToNextSentence);
        assert_eq!(e.text_lines()[0], "");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.register_text(), "First sentence. Second here.\n");
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn yank_sentence_keeps_cursor_at_start() {
        let mut e = editor_with(&sent_text());
        e.apply(Action::JumpTo(0, 11));
        e.apply(Action::YankToNextSentence);
        assert_eq!(e.register_text(), "nce. ");
        assert_eq!((e.row(), e.col()), (0, 11));
    }

    #[test]
    fn change_sentence_enters_insert_at_start() {
        let mut e = editor_with(&sent_text());
        e.apply(Action::JumpTo(0, 17));
        e.apply(Action::ChangeToNextSentence);
        assert_eq!(e.text_lines()[0], "First sentence. S");
        assert_eq!((e.row(), e.col()), (0, 17));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn visual_sentence_extends_head() {
        let mut e = editor_with(&sent_text());
        e.apply(Action::JumpTo(0, 11));
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveToNextSentence);
        assert_eq!(e.selection_range(), Some(((0, 11), (0, 16))));
        assert_eq!((e.row(), e.col()), (0, 16));
    }

    #[test]
    fn delete_next_paragraph() {
        let mut e = editor_with(&para_text());
        e.apply(Action::DeleteToNextParagraph);
        assert_eq!(e.text_lines()[0], "");
        assert_eq!(e.text_lines()[1], "para two");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.register_text(), "para one line1\npara one line2\n");
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn delete_prev_paragraph() {
        let mut e = editor_with(&para_text());
        e.apply(Action::JumpTo(1, 0));
        e.apply(Action::DeleteToPrevParagraph);
        assert_eq!(e.text_lines()[0], "para one line2");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.register_text(), "para one line1\n");
    }

    #[test]
    fn delete_paragraph_onto_blank_keeps_line() {
        let mut e = editor_with(&para_text());
        e.apply(Action::JumpTo(3, 0));
        e.apply(Action::DeleteToPrevParagraph);
        assert_eq!(e.text_lines()[2], "para two");
        assert_eq!((e.row(), e.col()), (2, 0));
        assert_eq!(e.register_text(), "\n");
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn delete_paragraph_at_eof() {
        let mut e = editor_with(&para_text());
        e.apply(Action::JumpTo(6, 0));
        e.apply(Action::DeleteToNextParagraph);
        assert_eq!(e.line_count(), 6);
        assert_eq!((e.row(), e.col()), (5, 0));
        assert_eq!(e.register_text(), "para three line1\npara three line2");
    }

    #[test]
    fn delete_paragraph_count() {
        let mut e = editor_with(&para_text());
        push_digits(&mut e, &[2]);
        e.apply(Action::DeleteToNextParagraph);
        assert_eq!(e.text_lines()[0], "");
        assert_eq!(e.text_lines()[1], "");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(
            e.register_text(),
            "para one line1\npara one line2\n\npara two\n"
        );
    }

    #[test]
    fn yank_paragraph_keeps_cursor_at_start() {
        let mut e = editor_with(&para_text());
        e.apply(Action::JumpTo(3, 0));
        e.apply(Action::YankToPrevParagraph);
        assert_eq!(e.register_text(), "\n");
        assert_eq!((e.row(), e.col()), (2, 0));
    }

    #[test]
    fn next_paragraph_stops_at_macro() {
        let mut e = editor_with(&macro_text());
        e.apply(Action::MoveToNextParagraph);
        assert_eq!((e.row(), e.col()), (1, 0));
        e.apply(Action::MoveToNextParagraph);
        assert_eq!((e.row(), e.col()), (3, 0));
    }

    #[test]
    fn next_paragraph_stops_at_section_macro() {
        let mut e = editor_with(&macro_text());
        e.apply(Action::JumpTo(3, 0));
        e.apply(Action::MoveToNextParagraph);
        assert_eq!((e.row(), e.col()), (5, 0));
    }

    #[test]
    fn delete_to_macro_paragraph_is_linewise() {
        let mut e = editor_with(&macro_text());
        e.apply(Action::JumpTo(3, 0));
        e.apply(Action::DeleteToNextParagraph);
        assert_eq!(
            e.text_lines(),
            vec![
                "intro line".to_string(),
                ".PP".to_string(),
                "para two".to_string(),
                ".SH NAME".to_string(),
                "tail".to_string()
            ]
        );
        assert_eq!((e.row(), e.col()), (3, 0));
        assert_eq!(e.register_text(), ".IP tag\npara three\n");
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn yank_to_last_line_macro_is_charwise() {
        let mut e = editor_with(&macro_tail_text());
        e.apply(Action::JumpTo(3, 0));
        e.apply(Action::YankToNextParagraph);
        assert_eq!(e.register_text(), ".P\nthree\n.p\nfour\n.PP");
        assert_eq!(e.register_kind(), RegisterKind::CharWise);
        assert_eq!((e.row(), e.col()), (3, 0));
    }

    #[test]
    fn delete_to_last_line_macro_is_linewise() {
        let mut e = editor_with(&macro_tail_text());
        e.apply(Action::JumpTo(3, 0));
        e.apply(Action::DeleteToNextParagraph);
        assert_eq!(
            e.text_lines(),
            vec![
                "one".to_string(),
                ".PPx rest".to_string(),
                "two".to_string(),
                String::new()
            ]
        );
        assert_eq!((e.row(), e.col()), (2, 0));
        assert_eq!(e.register_text(), ".P\nthree\n.p\nfour\n.PP");
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn next_paragraph_macro_on_last_line_lands_on_char() {
        let mut e = editor_with(&macro_tail_text());
        e.apply(Action::JumpTo(4, 0));
        e.apply(Action::MoveToNextParagraph);
        assert_eq!((e.row(), e.col()), (7, 2));
    }

    #[test]
    fn change_to_macro_paragraph_keeps_indent() {
        let mut e = editor_with("  one\n  two\n.PP\ntail");
        e.apply(Action::ChangeToNextParagraph);
        assert_eq!(e.text_lines()[0], "  ");
        assert_eq!((e.row(), e.col()), (0, 2));
        assert_eq!(e.mode(), Mode::Insert);
        e.apply(Action::InsertChar('Z'));
        assert_eq!(e.text_lines()[0], "  Z");
        assert_eq!(e.register_text(), "  one\n  two");
    }

    #[test]
    fn change_paragraph_keeps_indent() {
        let mut e = editor_with("    ind one\n    ind two\n\n    ind three\n");
        e.apply(Action::ChangeToNextParagraph);
        assert_eq!(e.text_lines()[0], "    ");
        assert_eq!((e.row(), e.col()), (0, 4));
        assert_eq!(e.mode(), Mode::Insert);
        assert_eq!(e.register_text(), "    ind one\n    ind two");
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
        e.apply(Action::InsertChar('Z'));
        assert_eq!(e.text_lines()[0], "    Z");
    }

    #[test]
    fn delete_paragraph_indented() {
        let mut e = editor_with("    ind one\n    ind two\n\n    ind three\n");
        e.apply(Action::DeleteToNextParagraph);
        assert_eq!(e.text_lines()[0], "");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_paragraph_from_indent_is_linewise() {
        let mut e = editor_with("    ind one\n    ind two\n\n    ind three\n");
        e.apply(Action::JumpTo(0, 1));
        e.apply(Action::DeleteToNextParagraph);
        assert_eq!(e.text_lines()[0], "");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.register_text(), "    ind one\n    ind two\n");
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn yank_paragraph_from_midline_backs_up() {
        let mut e = editor_with(&para_text());
        e.apply(Action::JumpTo(0, 5));
        e.apply(Action::YankToNextParagraph);
        assert_eq!(e.register_text(), "one line1\npara one line2");
        assert_eq!(e.register_kind(), RegisterKind::CharWise);
        assert_eq!((e.row(), e.col()), (0, 5));
    }

    #[test]
    fn change_paragraph_from_midline_backs_up() {
        let mut e = editor_with(&para_text());
        e.apply(Action::JumpTo(0, 5));
        e.apply(Action::ChangeToNextParagraph);
        assert_eq!(e.text_lines()[0], "para ");
        assert_eq!(e.text_lines()[1], "");
        assert_eq!(e.register_text(), "one line1\npara one line2");
        assert_eq!(e.register_kind(), RegisterKind::CharWise);
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn visual_paragraph_extends_head() {
        let mut e = editor_with(&para_text());
        e.apply(Action::JumpTo(1, 0));
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveToNextParagraph);
        assert_eq!(e.selection_range(), Some(((1, 0), (2, 0))));
        assert_eq!((e.row(), e.col()), (2, 0));
    }

    #[test]
    fn delete_sentence_eol_next_linewise() {
        let mut e = editor_with("aaa.\nbbb\n");
        e.apply(Action::DeleteToNextSentence);
        assert_eq!(e.text_lines()[0], "bbb");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.register_text(), "aaa.\n");
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }

    #[test]
    fn delete_sentence_backward_col0_linewise() {
        let mut e = editor_with(&sent_text());
        e.apply(Action::JumpTo(4, 0));
        e.apply(Action::DeleteToPrevSentence);
        assert_eq!(e.text_lines()[3], "(Paren.) \"Quoted.\" Atas.");
        assert_eq!((e.row(), e.col()), (3, 0));
        assert_eq!(e.register_text(), "No terminator here\n");
        assert_eq!(e.register_kind(), RegisterKind::LineWise);
    }
}
