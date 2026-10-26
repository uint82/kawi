use kawi_buffer::Buffer;

use super::bracket_match::{find_enclosing, MatchRequest};
use crate::editing::Pair;
use crate::words::{dec_pos, inc_pos, resolve_exclusive, resolve_inclusive, WordSpan};

pub fn bracket_range(
    request: &MatchRequest<'_>,
    pair: Pair,
    inner: bool,
    count: usize,
) -> Option<WordSpan> {
    let matched = find_enclosing(request, pair, count)?;
    let start = matched.first;
    let end = matched.second;
    if !inner {
        return Some(resolve_inclusive(request.buffer, start, end, true));
    }
    let (inner_start, inner_end, sol) = exclude_brackets(request.buffer, start, end);
    if inner_start > inner_end {
        return Some(WordSpan::Empty { at: inner_start });
    }
    if sol {
        return Some(resolve_exclusive(request.buffer, inner_start, inner_end));
    }
    Some(WordSpan::Charwise {
        start: inner_start,
        end: inner_end,
    })
}

fn exclude_brackets(
    buffer: &Buffer,
    open: (usize, usize),
    close: (usize, usize),
) -> ((usize, usize), (usize, usize), bool) {
    let start = incl_pos(buffer, open.0, open.1);
    let mut end = decl_pos(buffer, close.0, close.1);
    let mut sol = close.1 == 0;
    while strictly_before_nonblank(buffer, end.0, end.1) {
        sol = true;
        let (next, step) = dec_pos(buffer, end.0, end.1);
        end = next;
        if step != 0 {
            break;
        }
    }
    if sol {
        end = incl_pos(buffer, end.0, end.1);
    }
    (start, end, sol)
}

fn incl_pos(buffer: &Buffer, row: usize, col: usize) -> (usize, usize) {
    let (mut pos, step) = inc_pos(buffer, row, col);
    if step >= 1 && pos.1 != 0 {
        (pos, _) = inc_pos(buffer, pos.0, pos.1);
    }
    pos
}

fn decl_pos(buffer: &Buffer, row: usize, col: usize) -> (usize, usize) {
    let (mut pos, step) = dec_pos(buffer, row, col);
    if step == 1 && pos.1 != 0 {
        (pos, _) = dec_pos(buffer, pos.0, pos.1);
    }
    pos
}

fn strictly_before_nonblank(buffer: &Buffer, row: usize, col: usize) -> bool {
    match buffer.line_cow(row) {
        None => false,
        Some(line) => col.saturating_add(1) <= kawi_buffer::first_non_blank(&line),
    }
}

#[cfg(test)]
mod tests;
