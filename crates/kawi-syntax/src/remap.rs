use std::ops::Range;

use ropey::Rope;

use crate::span::{SyntaxEdit, SyntaxSpan};
use crate::text;

pub fn shift_spans(
    spans: &[SyntaxSpan],
    old_text: &Rope,
    new_text: &Rope,
    edits: &[SyntaxEdit],
) -> Vec<SyntaxSpan> {
    if edits.is_empty() {
        return spans.to_vec();
    }
    spans
        .iter()
        .filter_map(|span| remap_span(*span, old_text, new_text, edits))
        .collect()
}

pub(crate) fn remap_span(
    span: SyntaxSpan,
    old_text: &Rope,
    new_text: &Rope,
    edits: &[SyntaxEdit],
) -> Option<SyntaxSpan> {
    let old_chars = span_chars(span, old_text)?;
    let new_chars = apply_edits(old_chars.clone(), edits)?;
    let old_bytes = text::byte_range(old_text, old_chars.clone());
    let new_bytes = text::byte_range(new_text, new_chars.clone());
    if old_text.get_byte_slice(old_bytes.clone())? != new_text.get_byte_slice(new_bytes.clone())? {
        return None;
    }
    if !is_token_boundary(old_text, old_bytes) || !is_token_boundary(new_text, new_bytes.clone()) {
        return None;
    }
    let start = text::point(new_text, new_bytes.start);
    let end = text::point(new_text, new_bytes.end);
    if start.row != end.row {
        return None;
    }
    let line_start = text::line_start(new_text, start.row);
    let start_column = text::char_column(new_text, line_start, new_bytes.start);
    let end_column = text::char_column(new_text, line_start, new_bytes.end);
    if start_column >= end_column {
        return None;
    }
    Some(SyntaxSpan {
        row: start.row,
        start: start_column,
        end: end_column,
        highlight: span.highlight,
    })
}

fn span_chars(span: SyntaxSpan, text: &Rope) -> Option<Range<usize>> {
    if span.row >= text.len_lines() {
        return None;
    }
    let line_start = text.line_to_char(span.row);
    let line_len = text::row_char_len(text, span.row);
    let start = line_start.saturating_add(span.start.min(line_len));
    let end = line_start.saturating_add(span.end.min(line_len));
    (start < end).then_some(start..end)
}

fn apply_edits(mut span: Range<usize>, edits: &[SyntaxEdit]) -> Option<Range<usize>> {
    for edit in edits {
        if span.end <= edit.start {
            continue;
        }
        if span.start >= edit.end {
            let delta = edit.text.chars().count() as isize - (edit.end - edit.start) as isize;
            span.start = span.start.checked_add_signed(delta)?;
            span.end = span.end.checked_add_signed(delta)?;
            continue;
        }
        return None;
    }
    Some(span)
}

fn is_token_boundary(text: &Rope, bytes: Range<usize>) -> bool {
    let before = char_before(text, bytes.start);
    let first = char_at(text, bytes.start);
    let last = char_before(text, bytes.end);
    let after = char_at(text, bytes.end);
    !(is_word(before, first) || is_word(last, after))
}

fn char_at(text: &Rope, byte: usize) -> Option<char> {
    text.get_char(text.try_byte_to_char(byte).ok()?)
}

fn char_before(text: &Rope, byte: usize) -> Option<char> {
    let index = text.try_byte_to_char(byte).ok()?.checked_sub(1)?;
    text.get_char(index)
}

fn is_word(left: Option<char>, right: Option<char>) -> bool {
    left.is_some_and(is_identifier_char) && right.is_some_and(is_identifier_char)
}

fn is_identifier_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::SyntaxHighlight;

    fn span(row: usize, start: usize, end: usize) -> SyntaxSpan {
        SyntaxSpan {
            row,
            start,
            end,
            highlight: SyntaxHighlight(0),
        }
    }

    fn edit(start: usize, end: usize, text: &str) -> SyntaxEdit {
        SyntaxEdit {
            start,
            end,
            text: text.to_string(),
        }
    }

    fn shifted(
        spans: &[SyntaxSpan],
        old: &str,
        new: &str,
        edits: &[SyntaxEdit],
    ) -> Vec<SyntaxSpan> {
        shift_spans(spans, &Rope::from_str(old), &Rope::from_str(new), edits)
    }

    #[test]
    fn shifts_spans_after_an_insertion() {
        let spans = [span(0, 4, 7)];
        assert_eq!(
            shifted(&spans, "foo bar", "foo  bar", &[edit(4, 4, " ")]),
            vec![span(0, 5, 8)]
        );
    }

    #[test]
    fn leaves_spans_before_an_insertion_alone() {
        let spans = [span(0, 0, 3)];
        assert_eq!(
            shifted(&spans, "foo bar", "foo Xbar", &[edit(3, 3, " ")]),
            vec![span(0, 0, 3)]
        );
    }

    #[test]
    fn drops_spans_that_contain_the_edit() {
        let spans = [span(0, 4, 7)];
        assert!(shifted(&spans, "foo bar", "foo baz", &[edit(6, 7, "z")]).is_empty());
    }

    #[test]
    fn shifts_spans_after_a_deletion() {
        let spans = [span(0, 8, 13)];
        assert_eq!(
            shifted(&spans, "one two three", "one three", &[edit(3, 7, "")]),
            vec![span(0, 4, 9)]
        );
    }

    #[test]
    fn moves_spans_between_rows() {
        let spans = [span(1, 0, 4)];
        assert_eq!(
            shifted(
                &spans,
                "alpha\nbeta\n",
                "\nalpha\nbeta\n",
                &[edit(0, 0, "\n")]
            ),
            vec![span(2, 0, 4)]
        );
    }

    #[test]
    fn shifts_unicode_columns_by_characters() {
        let spans = [span(0, 2, 3)];
        assert_eq!(
            shifted(&spans, "é-x", "éé-x", &[edit(0, 0, "é")]),
            vec![span(0, 3, 4)]
        );
    }

    #[test]
    fn applies_multiple_edits_in_order() {
        let spans = [span(0, 4, 7)];
        assert_eq!(
            shifted(
                &spans,
                "foo bar baz",
                "Xfoo bar bazY",
                &[edit(0, 0, "X"), edit(11, 11, "Y")]
            ),
            vec![span(0, 5, 8)]
        );
    }

    #[test]
    fn drops_spans_joined_to_neighbours() {
        let spans = [span(0, 4, 7)];
        assert!(shifted(&spans, "foo bar", "foobar", &[edit(3, 4, "")]).is_empty());
        assert_eq!(
            shifted(&spans, "foo bar", "foox bar", &[edit(3, 3, "x")]),
            vec![span(0, 5, 8)]
        );
        assert!(shifted(&spans, "foo bar", "foo xbar", &[edit(4, 4, "x")]).is_empty());
    }

    #[test]
    fn drops_spans_when_their_text_changes_elsewhere() {
        let spans = [span(0, 0, 3)];
        assert!(shifted(&spans, "abc def", "abX def", &[edit(2, 3, "X")]).is_empty());
    }
}
