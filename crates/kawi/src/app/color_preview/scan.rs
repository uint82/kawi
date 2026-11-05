use std::ops::Range;

use crossterm::style::Color;
use kawi_tui::frame::DecorationSpan;

use super::color::Token;
use super::context::{self, MarkupState};
use super::hex;
use super::mode::PreviewMode;
use super::named;
use super::rgb;

const HASH: u8 = b'#';
const OPEN_PAREN: u8 = b'(';
const BACKSLASH: u8 = b'\\';

pub(super) fn spans(
    lines: &[String],
    rows: &[usize],
    background: Color,
    mode: PreviewMode,
) -> Vec<DecorationSpan> {
    let mut spans = Vec::new();
    let mut state = MarkupState::default();
    let mut previous_row = None;
    for (line, row) in lines.iter().zip(rows.iter().copied()) {
        if previous_row.is_some_and(|previous: usize| row != previous.saturating_add(1)) {
            state = MarkupState::default();
        }
        previous_row = Some(row);
        let allowed = context::preview_ranges(line, mode, &mut state);
        scan_line(row, line, background, &allowed, &mut spans);
    }
    spans
}

fn scan_line(
    row: usize,
    line: &str,
    background: Color,
    allowed: &[Range<usize>],
    spans: &mut Vec<DecorationSpan>,
) {
    let bytes = line.as_bytes();
    let mut column = 0;
    for (index, character) in line.char_indices() {
        if is_boundary_before(bytes, index) {
            if let Some(token) = token_at(line, bytes, index, character, allowed) {
                push_token(spans, row, column, token, background);
            }
        }
        column = column.saturating_add(1);
    }
}

fn token_at(
    line: &str,
    bytes: &[u8],
    index: usize,
    character: char,
    allowed: &[Range<usize>],
) -> Option<Token> {
    if character == '#' {
        return hex::token_at(bytes, index)
            .filter(|token| is_boundary_after(bytes, index.saturating_add(token.len)));
    }
    if matches!(character, 'r' | 'R') && is_allowed(allowed, index) {
        if let Some(token) = rgb::token_at(bytes, index)
            .filter(|token| is_boundary_after(bytes, index.saturating_add(token.len)))
        {
            return Some(token);
        }
    }
    if character.is_ascii_alphabetic() && is_allowed(allowed, index) {
        return named_at(line, bytes, index);
    }
    None
}

fn named_at(line: &str, bytes: &[u8], index: usize) -> Option<Token> {
    let end = word_end(bytes, index);
    if !is_boundary_after(bytes, end) || bytes.get(end) == Some(&OPEN_PAREN) {
        return None;
    }
    let color = named::lookup(line.get(index..end)?)?;
    Some(Token::new(color, end.saturating_sub(index)))
}

fn word_end(bytes: &[u8], start: usize) -> usize {
    bytes
        .get(start..)
        .map(|tail| {
            tail.iter()
                .take_while(|byte| byte.is_ascii_alphabetic())
                .count()
        })
        .unwrap_or_default()
        .saturating_add(start)
}

fn is_allowed(ranges: &[Range<usize>], index: usize) -> bool {
    ranges
        .iter()
        .any(|range| range.start <= index && index < range.end)
}

fn is_boundary_before(bytes: &[u8], index: usize) -> bool {
    index
        .checked_sub(1)
        .and_then(|previous| bytes.get(previous))
        .is_none_or(|byte| is_boundary(*byte))
}

fn is_boundary_after(bytes: &[u8], index: usize) -> bool {
    bytes.get(index).is_none_or(|byte| is_boundary(*byte))
}

fn is_boundary(byte: u8) -> bool {
    !byte.is_ascii_alphanumeric()
        && byte != b'_'
        && byte != b'-'
        && byte != HASH
        && byte != BACKSLASH
}

fn push_token(
    spans: &mut Vec<DecorationSpan>,
    row: usize,
    column: usize,
    token: Token,
    background: Color,
) {
    spans.push(DecorationSpan {
        row,
        start: column,
        end: column.saturating_add(token.len),
        style: token.color.to_style(background),
    });
}
