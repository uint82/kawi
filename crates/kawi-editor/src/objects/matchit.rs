use kawi_buffer::Buffer;

use super::smartmatch::{find_close_forward, find_open_backward, starts_escaped, MatchConfig};
use crate::words::char_at;

pub enum MatchTarget {
    Bracket { pos: (usize, usize) },
    Hash { line: usize, forward: bool },
    Comment { pos: (usize, usize) },
}

const PAIRS: [(char, char); 3] = [('(', ')'), ('{', '}'), ('[', ']')];

// TODO(matchit): nvim loads the matchit plugin by default and maps percent to
// a syntax-aware search: candidates in string, character, singlequote, escape,
// symbol or comment syntax regions are skipped, then b:match_words keyword
// pairs are matched per filetype. Needs a syntax model or treesitter captures.
pub fn match_target(buffer: &Buffer, row: usize, col: usize) -> Option<MatchTarget> {
    if let Some(found) = hash_jump_before_hash(buffer, row, col) {
        return Some(found);
    }
    if let Some(found) = comment_jump(buffer, row, col) {
        return Some(found);
    }
    if let Some(found) = bracket_jump(buffer, row, col) {
        return Some(found);
    }
    hash_jump_anywhere(buffer, row)
}

fn hash_jump_before_hash(buffer: &Buffer, row: usize, col: usize) -> Option<MatchTarget> {
    let hash = hash_column(buffer, row)?;
    if col > hash {
        return None;
    }
    hash_directive_target(buffer, row)
}

fn hash_jump_anywhere(buffer: &Buffer, row: usize) -> Option<MatchTarget> {
    hash_column(buffer, row)?;
    hash_directive_target(buffer, row)
}

fn hash_column(buffer: &Buffer, row: usize) -> Option<usize> {
    let line = buffer.line_cow(row)?;
    let text: Vec<char> = line.chars().collect();
    let hash = text.iter().position(|ch| !ch.is_whitespace())?;
    (text.get(hash) == Some(&'#')).then_some(hash)
}

fn hash_directive_target(buffer: &Buffer, row: usize) -> Option<MatchTarget> {
    let kind = hash_line(buffer, row)?;
    let forward = !matches!(kind, HashKind::Endif);
    hash_match(buffer, row, forward).map(|line| MatchTarget::Hash { line, forward })
}

#[derive(PartialEq, Eq)]
enum HashKind {
    If,
    Else,
    Endif,
}

fn hash_directive(text: &[char]) -> Option<HashKind> {
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
    if word.starts_with("endif") {
        Some(HashKind::Endif)
    } else if word.starts_with("if") || word.starts_with("el") {
        if word.starts_with("el") {
            Some(HashKind::Else)
        } else {
            Some(HashKind::If)
        }
    } else {
        None
    }
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
        let Some(kind) = hash_line(buffer, line) else {
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

fn hash_line(buffer: &Buffer, row: usize) -> Option<HashKind> {
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
            let config = MatchConfig::smart(open, close, starts_escaped(buffer, (row, at)));
            let found = if forward {
                find_close_forward(buffer, (row, at), config)
            } else {
                find_open_backward(buffer, (row, at), config)
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
