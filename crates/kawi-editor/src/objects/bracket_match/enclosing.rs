use crate::editor::lexical::{LexicalContext, LexicalScanner};

use super::{
    context_start, find_match, is_escaped, line_chars, BracketMatch, MatchRequest, Position,
    LEXICAL_CONTEXT_PADDING,
};

#[derive(Clone, Copy)]
struct OpenBracket {
    position: Position,
}

pub fn find_enclosing(
    request: &MatchRequest<'_>,
    pair: crate::editing::Pair,
    count: usize,
) -> Option<BracketMatch> {
    if count == 0 || pair.open == pair.close {
        return None;
    }
    let (row, col) = request.position;
    let start_row = context_start(
        request.buffer,
        row,
        col,
        request.bound.saturating_add(LEXICAL_CONTEXT_PADDING),
    );
    let mut scanner = LexicalScanner::new(LexicalContext::with_language(
        request.editing,
        request.language,
    ));
    let mut code_opens: Vec<OpenBracket> = Vec::new();
    let mut other_opens: Vec<OpenBracket> = Vec::new();
    let mut closed_at_cursor = None;
    let mut cursor_code = false;
    for current_row in start_row..=row {
        let chars = line_chars(request.buffer, current_row)?;
        let end = if current_row == row {
            col.min(chars.len().saturating_sub(1))
        } else {
            chars.len().saturating_sub(1)
        };
        if chars.is_empty() {
            if current_row < row {
                scanner.finish_line();
            }
            continue;
        }
        for index in 0..=end {
            let was_code = scanner.in_code();
            if current_row == row && index == col {
                cursor_code = was_code;
            }
            let ch = chars[index];
            let escaped = is_escaped(&chars, index);
            if !escaped && ch == pair.close {
                let opens = opens_for(was_code, &mut code_opens, &mut other_opens);
                if let Some(found) = opens.pop() {
                    if current_row == row && index == col {
                        closed_at_cursor = Some(found);
                    }
                }
            } else if !escaped && ch == pair.open {
                opens_for(was_code, &mut code_opens, &mut other_opens).push(OpenBracket {
                    position: (current_row, index),
                });
            }
            scanner.advance(&chars, index);
        }
        if current_row < row {
            scanner.finish_line();
        }
    }
    let enclosing = enclosing_candidates(cursor_code, closed_at_cursor, &code_opens, &other_opens);
    if !enclosing.is_empty() {
        return enclosing
            .into_iter()
            .filter_map(|found| matched_pair(request, found.position))
            .nth(count.saturating_sub(1));
    }
    let next = next_opens(&mut scanner, request, row, col, pair, count)?;
    next.into_iter()
        .filter_map(|found| matched_pair(request, found.position))
        .nth(count.saturating_sub(1))
}

fn opens_for<'a>(
    code: bool,
    code_opens: &'a mut Vec<OpenBracket>,
    other_opens: &'a mut Vec<OpenBracket>,
) -> &'a mut Vec<OpenBracket> {
    if code {
        code_opens
    } else {
        other_opens
    }
}

fn enclosing_candidates(
    cursor_code: bool,
    closed_at_cursor: Option<OpenBracket>,
    code_opens: &[OpenBracket],
    other_opens: &[OpenBracket],
) -> Vec<OpenBracket> {
    let (same, other) = if cursor_code {
        (code_opens, other_opens)
    } else {
        (other_opens, code_opens)
    };
    let mut found = Vec::new();
    if let Some(closed) = closed_at_cursor {
        found.push(closed);
    }
    found.extend(same.iter().rev().copied());
    found.extend(other.iter().rev().copied());
    found
}

fn matched_pair(request: &MatchRequest<'_>, open: Position) -> Option<BracketMatch> {
    find_match(&MatchRequest {
        buffer: request.buffer,
        position: open,
        language: request.language,
        editing: request.editing,
        pairs: request.pairs,
        bound: request.bound,
    })
    .map(|close| BracketMatch {
        first: open,
        second: close,
    })
}

fn next_opens(
    scanner: &mut LexicalScanner,
    request: &MatchRequest<'_>,
    row: usize,
    col: usize,
    pair: crate::editing::Pair,
    count: usize,
) -> Option<Vec<OpenBracket>> {
    let mut found = Vec::new();
    let mut offset = 0usize;
    for current_row in row..request.buffer.line_count() {
        let chars = line_chars(request.buffer, current_row)?;
        let start = if current_row == row {
            col.saturating_add(1)
        } else {
            0
        };
        for index in start..chars.len() {
            scanner.advance(&chars, index);
            if chars[index] == pair.open && !is_escaped(&chars, index) {
                found.push(OpenBracket {
                    position: (current_row, index),
                });
                if found.len() == count {
                    return Some(found);
                }
            }
            offset = offset.saturating_add(1);
            if offset > request.bound {
                return None;
            }
        }
        scanner.finish_line();
    }
    None
}
