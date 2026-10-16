use kawi_buffer::Buffer;

use super::brackets::{find_close_forward, find_open_backward};
use crate::words::char_at;

pub enum MatchTarget {
    Bracket { pos: (usize, usize) },
    Hash { line: usize, forward: bool },
    Comment { pos: (usize, usize) },
}

const PAIRS: [(char, char); 3] = [('(', ')'), ('{', '}'), ('[', ']')];

pub fn match_target(buffer: &Buffer, row: usize, col: usize) -> Option<MatchTarget> {
    if let Some(found) = hash_jump(buffer, row, col) {
        return Some(found);
    }
    if let Some(found) = comment_jump(buffer, row, col) {
        return Some(found);
    }
    bracket_jump(buffer, row, col)
}

fn hash_jump(buffer: &Buffer, row: usize, col: usize) -> Option<MatchTarget> {
    let line = buffer.line_cow(row)?;
    let text: Vec<char> = line.chars().collect();
    let col = col.min(text.len().saturating_sub(1));
    let (end, kind) = hash_directive(&text)?;
    if col > end {
        return None;
    }
    let forward = !matches!(kind, HashKind::Endif);
    hash_match(buffer, row, forward).map(|line| MatchTarget::Hash { line, forward })
}

#[derive(PartialEq, Eq)]
enum HashKind {
    If,
    Else,
    Endif,
}

fn hash_directive(text: &[char]) -> Option<(usize, HashKind)> {
    let hash = text.iter().position(|ch| !ch.is_whitespace())?;
    if text.get(hash) != Some(&'#') {
        return None;
    }
    let start = hash.saturating_add(1);
    let mut end = start;
    while end < text.len() && !text[end].is_whitespace() {
        end = end.saturating_add(1);
    }
    let word: String = text[start..end].iter().collect();
    let kind = if word.starts_with("endif") {
        HashKind::Endif
    } else if word.starts_with("if") || word.starts_with("el") {
        if word.starts_with("el") {
            HashKind::Else
        } else {
            HashKind::If
        }
    } else {
        return None;
    };
    Some((end.saturating_sub(1), kind))
}

fn hash_match(buffer: &Buffer, row: usize, forward: bool) -> Option<usize> {
    let last = buffer.line_count().saturating_sub(1);
    let mut line = row;
    let mut depth: usize = 0;
    loop {
        if forward {
            if line >= last {
                return None;
            }
            line = line.saturating_add(1);
        } else {
            if line == 0 {
                return None;
            }
            line = line.saturating_sub(1);
        }
        let Some((_, kind)) = hash_line(buffer, line) else {
            continue;
        };
        match (kind, forward) {
            (HashKind::If, true) | (HashKind::Endif, false) => {
                depth = depth.saturating_add(1);
            }
            (HashKind::Else, _) => {
                if depth == 0 && forward {
                    return Some(line);
                }
            }
            _ => {
                if depth == 0 {
                    return Some(line);
                }
                depth = depth.saturating_sub(1);
            }
        }
    }
}

fn hash_line(buffer: &Buffer, row: usize) -> Option<(usize, HashKind)> {
    let line = buffer.line_cow(row)?;
    hash_directive(&line.chars().collect::<Vec<_>>())
}

fn comment_jump(buffer: &Buffer, row: usize, col: usize) -> Option<MatchTarget> {
    let line = buffer.line_cow(row)?;
    let text: Vec<char> = line.chars().collect();
    if text.get(col) == Some(&'/') && text.get(col.saturating_add(1)) == Some(&'*') {
        return comment_match(buffer, row, col, true).map(|pos| MatchTarget::Comment { pos });
    }
    if text.get(col) == Some(&'*') && text.get(col.saturating_sub(1)) == Some(&'/') {
        return comment_match(buffer, row, col, true).map(|pos| MatchTarget::Comment { pos });
    }
    if text.get(col) == Some(&'*') && text.get(col.saturating_add(1)) == Some(&'/') {
        return comment_match(buffer, row, col, false).map(|pos| MatchTarget::Comment { pos });
    }
    if text.get(col) == Some(&'/') && col > 0 && text.get(col.saturating_sub(1)) == Some(&'*') {
        return comment_match(buffer, row, col.saturating_sub(1), false)
            .map(|pos| MatchTarget::Comment { pos });
    }
    None
}

fn comment_match(buffer: &Buffer, row: usize, col: usize, forward: bool) -> Option<(usize, usize)> {
    let (mut row, mut col) = (row, col);
    loop {
        if forward {
            let (next, step) = crate::words::inc_pos(buffer, row, col);
            if step == -1 {
                return None;
            }
            row = next.0;
            col = next.1;
            if char_at(buffer, row, col) == Some('*')
                && char_at(buffer, row, col.saturating_add(1)) == Some('/')
            {
                return Some((row, col.saturating_add(1)));
            }
        } else {
            let (next, step) = crate::words::dec_pos(buffer, row, col);
            if step == -1 {
                return None;
            }
            row = next.0;
            col = next.1;
            if char_at(buffer, row, col) == Some('/')
                && char_at(buffer, row, col.saturating_add(1)) == Some('*')
            {
                return Some((row, col));
            }
        }
    }
}

fn bracket_jump(buffer: &Buffer, row: usize, col: usize) -> Option<MatchTarget> {
    let line = buffer.line_cow(row)?;
    let text: Vec<char> = line.chars().collect();
    if text.is_empty() {
        return None;
    }
    let col = col.min(text.len().saturating_sub(1));
    for (at, ch) in text.iter().enumerate().skip(col) {
        if let Some((open, close, forward)) = pair_of(*ch) {
            let found = if forward {
                find_close_forward(buffer, row, at, open, close)
            } else {
                find_open_backward(buffer, row, at, open, close)
            };
            return found.map(|pos| MatchTarget::Bracket { pos });
        }
    }
    None
}

fn pair_of(ch: char) -> Option<(char, char, bool)> {
    PAIRS
        .iter()
        .find(|(open, close)| *open == ch || *close == ch)
        .map(|(open, close)| (*open, *close, *open == ch))
}
