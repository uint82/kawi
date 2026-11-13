use std::ops::Range;

use ropey::Rope;

use crate::span::{SyntaxEdit, SyntaxSpan};
use crate::text;

pub(super) fn remap_span(
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
