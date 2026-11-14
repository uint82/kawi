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
    let mapper = SpanMapper::optimistic(old_text, new_text, edits);
    spans
        .iter()
        .filter_map(|span| mapper.remap(*span))
        .collect()
}

pub(crate) struct SpanMapper<'a> {
    old_text: &'a Rope,
    new_text: &'a Rope,
    edits: Vec<EditDelta>,
    mode: Mode,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Optimistic,
    Exact,
}

impl<'a> SpanMapper<'a> {
    pub(crate) fn optimistic(old_text: &'a Rope, new_text: &'a Rope, edits: &[SyntaxEdit]) -> Self {
        Self::new(old_text, new_text, edits, Mode::Optimistic)
    }

    pub(crate) fn exact(old_text: &'a Rope, new_text: &'a Rope, edits: &[SyntaxEdit]) -> Self {
        Self::new(old_text, new_text, edits, Mode::Exact)
    }

    fn new(old_text: &'a Rope, new_text: &'a Rope, edits: &[SyntaxEdit], mode: Mode) -> Self {
        Self {
            old_text,
            new_text,
            edits: edits.iter().map(EditDelta::from).collect(),
            mode,
        }
    }

    pub(crate) fn remap(&self, span: SyntaxSpan) -> Option<SyntaxSpan> {
        let old_chars = span_chars(span, self.old_text)?;
        let new_chars = match self.mode {
            Mode::Optimistic => apply_edits(old_chars.clone(), &self.edits)?,
            Mode::Exact => apply_exact_edits(old_chars.clone(), &self.edits)?,
        };
        let new_bytes = text::byte_range(self.new_text, new_chars);
        if self.mode == Mode::Exact {
            let old_bytes = text::byte_range(self.old_text, old_chars);
            if !self.unchanged(old_bytes, new_bytes.clone()) {
                return None;
            }
        }
        let start = text::point(self.new_text, new_bytes.start);
        let end = text::point(self.new_text, new_bytes.end);
        if start.row != end.row {
            return None;
        }
        let line_start = text::line_start(self.new_text, start.row);
        let start_column = text::char_column(self.new_text, line_start, new_bytes.start);
        let end_column = text::char_column(self.new_text, line_start, new_bytes.end);
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

    fn unchanged(&self, old: Range<usize>, new: Range<usize>) -> bool {
        let old_slice = self.old_text.get_byte_slice(old.clone());
        let new_slice = self.new_text.get_byte_slice(new.clone());
        old_slice.is_some_and(|old| new_slice.is_some_and(|new| old == new))
            && is_token_boundary(self.old_text, old)
            && is_token_boundary(self.new_text, new)
    }
}

#[derive(Clone, Copy)]
struct EditDelta {
    start: usize,
    end: usize,
    inserted: usize,
    has_line_break: bool,
}

impl From<&SyntaxEdit> for EditDelta {
    fn from(edit: &SyntaxEdit) -> Self {
        Self {
            start: edit.start,
            end: edit.end,
            inserted: edit.text.chars().count(),
            has_line_break: edit.text.contains('\n') || edit.text.contains('\r'),
        }
    }
}

#[derive(Clone, Copy)]
enum Gravity {
    Left,
    Right,
}

fn apply_edits(mut span: Range<usize>, edits: &[EditDelta]) -> Option<Range<usize>> {
    for edit in edits {
        let covered = span.start == edit.start && span.end == edit.end;
        let start_gravity = if covered {
            Gravity::Left
        } else {
            Gravity::Right
        };
        let end_gravity = if edit.has_line_break && !covered {
            Gravity::Left
        } else {
            Gravity::Right
        };
        span = map_position(span.start, edit, start_gravity)
            ..map_position(span.end, edit, end_gravity);
        if span.start >= span.end {
            return None;
        }
    }
    Some(span)
}

fn map_position(position: usize, edit: &EditDelta, gravity: Gravity) -> usize {
    if position < edit.start {
        return position;
    }
    if position > edit.end {
        return position - (edit.end - edit.start) + edit.inserted;
    }
    match gravity {
        Gravity::Left => edit.start,
        Gravity::Right => edit.start + edit.inserted,
    }
}

fn apply_exact_edits(mut span: Range<usize>, edits: &[EditDelta]) -> Option<Range<usize>> {
    for edit in edits {
        if span.end <= edit.start {
            continue;
        }
        if span.start >= edit.end {
            span.start = shifted_position(span.start, edit)?;
            span.end = shifted_position(span.end, edit)?;
            continue;
        }
        return None;
    }
    Some(span)
}

fn shifted_position(position: usize, edit: &EditDelta) -> Option<usize> {
    position
        .checked_sub(edit.end - edit.start)?
        .checked_add(edit.inserted)
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

#[cfg(test)]
mod tests;
