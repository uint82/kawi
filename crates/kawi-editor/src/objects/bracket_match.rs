use kawi_buffer::Buffer;
use kawi_syntax::LanguageId;

use crate::editing::{LanguageEditing, Pair};
use crate::editor::lexical::{LexicalContext, LexicalScanner};

pub const HIGHLIGHT_MATCH_STEP_BUDGET: usize = 4_096;
pub const ACTION_MATCH_BOUND: usize = 1_048_576;
pub const MAX_MATCH_LINE_CHARS: usize = 16_384;
const LEXICAL_CONTEXT_BUDGET: usize = 65_536;
pub(super) const LEXICAL_CONTEXT_PADDING: usize = 4_096;

pub type Position = (usize, usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BracketMatch {
    pub first: Position,
    pub second: Position,
}

pub struct MatchRequest<'a> {
    pub buffer: &'a Buffer,
    pub position: Position,
    pub language: Option<LanguageId>,
    pub editing: &'a LanguageEditing,
    pub pairs: &'a [Pair],
    pub bound: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MatchResult {
    pub partner: Position,
    pub in_code: bool,
}

mod enclosing;

pub use enclosing::find_enclosing;

pub fn find_match(request: &MatchRequest<'_>) -> Option<Position> {
    find_match_result(request).map(|result| result.partner)
}

pub(crate) fn find_match_result(request: &MatchRequest<'_>) -> Option<MatchResult> {
    if request.bound == 0 {
        return None;
    }
    let (row, col) = request.position;
    let chars = line_chars(request.buffer, row)?;
    let ch = *chars.get(col)?;
    let pair = request
        .pairs
        .iter()
        .copied()
        .find(|pair| pair.open == ch || pair.close == ch)?;
    if pair.open == pair.close {
        return None;
    }
    let target_escaped = is_escaped(&chars, col);
    let cursor_code = code_at(request, row, col)?;
    if ch == pair.open {
        find_forward(request, pair, row, col, cursor_code, target_escaped)
    } else {
        find_backward(request, pair, row, col, cursor_code, target_escaped)
    }
}

fn find_forward(
    request: &MatchRequest<'_>,
    pair: Pair,
    row: usize,
    col: usize,
    cursor_code: bool,
    target_escaped: bool,
) -> Option<MatchResult> {
    let start_row = context_start(request.buffer, row, col, LEXICAL_CONTEXT_BUDGET);
    let mut scanner = scanner(request);
    let mut offset = 0usize;
    let mut target_offset = None;
    let mut depth = 0usize;
    let mut started = false;
    for current_row in start_row..request.buffer.line_count() {
        let chars = line_chars(request.buffer, current_row)?;
        for index in 0..chars.len() {
            let code = scanner.advance(&chars, index);
            let position = (current_row, index);
            if position == (row, col) {
                started = true;
                target_offset = Some(offset);
                depth = 1;
            } else if started && eligible(&chars, index, code, cursor_code, target_escaped) {
                let ch = chars[index];
                if ch == pair.open {
                    depth = depth.saturating_add(1);
                } else if ch == pair.close {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        let distance = offset.saturating_sub(target_offset.unwrap_or(offset));
                        return (distance <= request.bound).then_some(MatchResult {
                            partner: position,
                            in_code: cursor_code,
                        });
                    }
                }
            }
            offset = offset.saturating_add(1);
            if target_offset.is_some_and(|target| offset.saturating_sub(target) > request.bound) {
                return None;
            }
        }
        scanner.finish_line();
    }
    None
}

fn find_backward(
    request: &MatchRequest<'_>,
    pair: Pair,
    row: usize,
    col: usize,
    cursor_code: bool,
    target_escaped: bool,
) -> Option<MatchResult> {
    let context_budget = request.bound.saturating_add(LEXICAL_CONTEXT_PADDING);
    let start_row = context_start(request.buffer, row, col, context_budget);
    let mut scanner = scanner(request);
    let mut stack: Vec<(Position, usize)> = Vec::new();
    let mut offset = 0usize;
    for current_row in start_row..=row {
        let chars = line_chars(request.buffer, current_row)?;
        if chars.is_empty() {
            if current_row < row {
                scanner.finish_line();
            }
            continue;
        }
        let end = if current_row == row {
            col
        } else {
            chars.len().saturating_sub(1)
        };
        for index in 0..=end {
            let code = scanner.advance(&chars, index);
            if eligible(&chars, index, code, cursor_code, target_escaped) {
                let ch = chars[index];
                if ch == pair.open {
                    stack.push(((current_row, index), offset));
                } else if ch == pair.close {
                    let Some((partner, partner_offset)) = stack.pop() else {
                        offset = offset.saturating_add(1);
                        continue;
                    };
                    if (current_row, index) == (row, col) {
                        let distance = offset.saturating_sub(partner_offset);
                        if distance <= request.bound {
                            return Some(MatchResult {
                                partner,
                                in_code: cursor_code,
                            });
                        }
                        return None;
                    }
                }
            }
            offset = offset.saturating_add(1);
        }
        if current_row < row {
            scanner.finish_line();
        }
    }
    None
}

fn code_at(request: &MatchRequest<'_>, row: usize, col: usize) -> Option<bool> {
    let start_row = context_start(
        request.buffer,
        row,
        col,
        LEXICAL_CONTEXT_BUDGET.min(request.bound.saturating_add(LEXICAL_CONTEXT_PADDING)),
    );
    let mut scanner = scanner(request);
    for current_row in start_row..=row {
        let chars = line_chars(request.buffer, current_row)?;
        for index in 0..chars.len() {
            let code = scanner.advance(&chars, index);
            if current_row == row && index == col {
                return Some(code);
            }
        }
        if current_row < row {
            scanner.finish_line();
        }
    }
    None
}

fn scanner(request: &MatchRequest<'_>) -> LexicalScanner {
    LexicalScanner::new(LexicalContext::with_language(
        request.editing,
        request.language,
    ))
}

fn eligible(
    chars: &[char],
    index: usize,
    code: bool,
    cursor_code: bool,
    target_escaped: bool,
) -> bool {
    code == cursor_code && is_escaped(chars, index) == target_escaped
}

pub(super) fn line_chars(buffer: &Buffer, row: usize) -> Option<Vec<char>> {
    let chars: Vec<char> = buffer.line_cow(row)?.chars().collect();
    (chars.len() <= MAX_MATCH_LINE_CHARS).then_some(chars)
}

pub(super) fn context_start(buffer: &Buffer, row: usize, mut col: usize, budget: usize) -> usize {
    let mut remaining = budget;
    let mut row = row;
    while remaining > 0 && row > 0 {
        if col > remaining {
            return row;
        }
        remaining = remaining.saturating_sub(col);
        row = row.saturating_sub(1);
        col = buffer.line_len(row);
    }
    if remaining == 0 {
        row
    } else {
        0
    }
}

pub(super) fn is_escaped(chars: &[char], index: usize) -> bool {
    let mut count = 0usize;
    let mut current = index;
    while current > 0 && chars.get(current.saturating_sub(1)) == Some(&'\\') {
        count = count.saturating_add(1);
        current = current.saturating_sub(1);
    }
    count % 2 == 1
}

#[cfg(test)]
mod tests;
