use std::collections::HashSet;
use std::ops::Range;

use tree_sitter::InputEdit;

use crate::diff::LineIndex;
use crate::rows;
use crate::span::SyntaxSpan;

pub(super) struct SpanMap<'a> {
    pub(super) old_text: &'a str,
    pub(super) old_index: &'a LineIndex,
    pub(super) new_text: &'a str,
    pub(super) new_index: &'a LineIndex,
    pub(super) edit: Option<&'a InputEdit>,
}

pub(super) fn remap(
    spans: &[SyntaxSpan],
    map: &SpanMap<'_>,
    keep: impl Fn(SyntaxSpan) -> Option<SyntaxSpan>,
) -> Vec<SyntaxSpan> {
    spans
        .iter()
        .filter_map(|span| map_span(span, map))
        .filter_map(keep)
        .collect()
}

pub(super) fn window_text<'a>(
    text: &'a str,
    index: &LineIndex,
    window: &Range<usize>,
) -> Option<&'a str> {
    let start = index.line_start(window.start);
    let end = index.line_start(window.end);
    text.get(start..end)
}

pub(super) fn local_span(span: SyntaxSpan, offset: usize) -> Option<SyntaxSpan> {
    Some(SyntaxSpan {
        row: span.row.checked_sub(offset)?,
        ..span
    })
}

pub(super) fn global_span(
    span: SyntaxSpan,
    window: &Range<usize>,
    error_rows: &HashSet<usize>,
) -> Option<SyntaxSpan> {
    let row = window.start.checked_add(span.row)?;
    keep_span(SyntaxSpan { row, ..span }, window, error_rows)
}

pub(super) fn keep_span(
    span: SyntaxSpan,
    window: &Range<usize>,
    error_rows: &HashSet<usize>,
) -> Option<SyntaxSpan> {
    (window.contains(&span.row) && error_rows.contains(&span.row)).then_some(span)
}

fn map_span(span: &SyntaxSpan, map: &SpanMap<'_>) -> Option<SyntaxSpan> {
    let old_start = char_byte(map.old_text, map.old_index, span.row, span.start)?;
    let old_end = char_byte(map.old_text, map.old_index, span.row, span.end)?;
    if !is_token_boundary(map.old_text, old_start, old_end) {
        return None;
    }
    let (new_start, new_end) = map_bytes(old_start, old_end, map.edit)?;
    let start = map.new_index.point(new_start);
    let end = map.new_index.point(new_end);
    if start.row != end.row {
        return None;
    }
    let line_start = map.new_index.line_start(start.row);
    let start_column = rows::char_column(map.new_text, line_start, new_start);
    let end_column = rows::char_column(map.new_text, line_start, new_end);
    if start_column >= end_column {
        return None;
    }
    if map.old_text.get(old_start..old_end)? != map.new_text.get(new_start..new_end)? {
        return None;
    }
    if !is_token_boundary(map.new_text, new_start, new_end) {
        return None;
    }
    Some(SyntaxSpan {
        row: start.row,
        start: start_column,
        end: end_column,
        highlight: span.highlight,
    })
}

fn is_token_boundary(text: &str, start: usize, end: usize) -> bool {
    let Some(span) = text.get(start..end) else {
        return false;
    };
    let before = text.get(..start).and_then(|text| text.chars().next_back());
    let first = span.chars().next();
    let last = span.chars().next_back();
    let after = text.get(end..).and_then(|text| text.chars().next());
    !(is_word(before, first) || is_word(last, after))
}

fn is_word(left: Option<char>, right: Option<char>) -> bool {
    left.is_some_and(is_identifier_char) && right.is_some_and(is_identifier_char)
}

fn is_identifier_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

fn char_byte(text: &str, index: &LineIndex, row: usize, column: usize) -> Option<usize> {
    if row >= index.line_count() {
        return None;
    }
    let start = index.line_start(row);
    let end = rows::line_text_end(text, index, row);
    let line = text.get(start..end)?;
    let offset = line
        .char_indices()
        .nth(column)
        .map(|(offset, _)| offset)
        .unwrap_or(line.len());
    Some(start + offset)
}

fn map_bytes(start: usize, end: usize, edit: Option<&InputEdit>) -> Option<(usize, usize)> {
    let Some(edit) = edit else {
        return Some((start, end));
    };
    if end <= edit.start_byte {
        return Some((start, end));
    }
    if start >= edit.old_end_byte {
        let delta = edit.new_end_byte as isize - edit.old_end_byte as isize;
        return Some((
            (start as isize + delta) as usize,
            (end as isize + delta) as usize,
        ));
    }
    None
}
