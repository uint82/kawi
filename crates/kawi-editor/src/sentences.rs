use kawi_buffer::Buffer;

use crate::words::{char_at, dec_pos, inc_pos};

pub(crate) struct ParagraphTarget {
    pub line: usize,
    pub col: usize,
    pub eof: bool,
}

pub(crate) fn sentence_target(
    buffer: &Buffer,
    row: usize,
    col: usize,
    forward: bool,
    count: usize,
) -> Option<(usize, usize)> {
    let mut pos = (row, col);
    let mut remaining = count.max(1);
    while remaining > 0 {
        let prev = pos;
        match step_sentence(buffer, pos, forward, remaining > 1) {
            Step::Fail => return None,
            Step::Done(next) => {
                if next == prev {
                    match advance(buffer, next, forward) {
                        Some(moved) => pos = moved,
                        None => return Some(next),
                    }
                } else {
                    pos = next;
                    remaining -= 1;
                }
            }
        }
    }
    Some(pos)
}

enum Step {
    Done((usize, usize)),
    Fail,
}

fn advance(buffer: &Buffer, pos: (usize, usize), forward: bool) -> Option<(usize, usize)> {
    if forward {
        let (next, step) = inc_pos(buffer, pos.0, pos.1);
        if step == -1 {
            return None;
        }
        Some(next)
    } else {
        let (next, step) = step_back(buffer, pos);
        if step == -1 {
            return None;
        }
        Some(next)
    }
}

fn step_back(buffer: &Buffer, pos: (usize, usize)) -> ((usize, usize), i32) {
    let (next, step) = dec_pos(buffer, pos.0, pos.1);
    if step == -1 {
        return (next, step);
    }
    let len = buffer.line_len(next.0);
    if len > 0 && next.1 >= len {
        return ((next.0, len.saturating_sub(1)), step);
    }
    (next, step)
}

fn step_sentence(buffer: &Buffer, pos: (usize, usize), forward: bool, more: bool) -> Step {
    let mut pos = pos;
    if is_eol(buffer, pos) {
        loop {
            match advance(buffer, pos, forward) {
                None => break,
                Some(next) => {
                    pos = next;
                    if !is_eol(buffer, pos) {
                        break;
                    }
                }
            }
        }
        if forward {
            return Step::Done(skip_spaces(buffer, pos));
        }
    } else if forward && pos.1 == 0 && is_para_start(buffer, pos.0) {
        if pos.0 >= buffer.line_count().saturating_sub(1) {
            return Step::Fail;
        }
        pos.0 = pos.0.saturating_add(1);
        return Step::Done(skip_spaces(buffer, pos));
    } else if !forward {
        let (next, step) = step_back(buffer, pos);
        if step != -1 {
            pos = next;
        }
    }
    skip_back(buffer, &mut pos, forward);
    let start_row = pos.0;
    match find_sentence_end(buffer, pos, start_row, forward, more) {
        Scan::Fail => Step::Fail,
        Scan::Found(found, true) => Step::Done(skip_spaces(buffer, found)),
        Scan::Found(found, false) => Step::Done(found),
    }
}

enum Scan {
    Found((usize, usize), bool),
    Fail,
}

fn find_sentence_end(
    buffer: &Buffer,
    mut pos: (usize, usize),
    start_row: usize,
    forward: bool,
    more: bool,
) -> Scan {
    let last_row = buffer.line_count().saturating_sub(1);
    loop {
        let c = char_at(buffer, pos.0, pos.1);
        if c.is_none() {
            if !forward {
                if pos.0 != start_row {
                    pos.0 = pos.0.saturating_add(1);
                }
                return Scan::Found(pos, true);
            }
            if pos.0 >= last_row || buffer.line_len(pos.0) == 0 {
                return Scan::Found(pos, true);
            }
            match advance(buffer, pos, true) {
                None => return Scan::Found(pos, true),
                Some(next) => {
                    pos = next;
                    continue;
                }
            }
        }
        if pos.1 == 0 && is_para_start(buffer, pos.0) {
            if !forward && pos.0 != start_row {
                pos.0 = pos.0.saturating_add(1);
            }
            return Scan::Found(pos, true);
        }
        if matches!(c, Some('.' | '!' | '?')) {
            if let Some(mut found) = sentence_end(buffer, pos) {
                if char_at(buffer, found.0, found.1).is_none() {
                    let (next, step) = inc_pos(buffer, found.0, found.1);
                    if step != -1 {
                        found = next;
                    }
                }
                return Scan::Found(found, true);
            }
        }
        match advance(buffer, pos, forward) {
            None => {
                if more {
                    return Scan::Fail;
                }
                return Scan::Found(pos, false);
            }
            Some(next) => pos = next,
        }
    }
}

fn is_eol(buffer: &Buffer, pos: (usize, usize)) -> bool {
    pos.1 >= buffer.line_len(pos.0)
}

fn is_para_start(buffer: &Buffer, row: usize) -> bool {
    // TODO(paragraph): nroff section and paragraph macros.
    match buffer.line_cow(row) {
        None => false,
        Some(line) => line.is_empty() || line.starts_with('\x0c'),
    }
}

fn skip_spaces(buffer: &Buffer, mut pos: (usize, usize)) -> (usize, usize) {
    while matches!(char_at(buffer, pos.0, pos.1), Some(' ' | '\t')) {
        let (next, step) = inc_pos(buffer, pos.0, pos.1);
        if step == -1 {
            break;
        }
        pos = next;
    }
    pos
}

fn skip_back(buffer: &Buffer, pos: &mut (usize, usize), forward: bool) {
    let mut found_dot = false;
    loop {
        let c = char_at(buffer, pos.0, pos.1);
        if !matches!(
            c,
            Some(' ' | '\t' | '.' | '!' | '?' | ')' | ']' | '"' | '\'')
        ) {
            break;
        }
        let c = c.unwrap_or('\0');
        let (prev, step) = step_back(buffer, *pos);
        if step == -1 || (is_empty_line(buffer, prev.0) && forward) {
            break;
        }
        if found_dot {
            break;
        }
        if matches!(c, '.' | '!' | '?') {
            found_dot = true;
        }
        if matches!(c, ')' | ']' | '"' | '\'')
            && !matches!(
                char_at(buffer, prev.0, prev.1),
                Some('.' | '!' | '?' | ')' | ']' | '"' | '\'')
            )
        {
            break;
        }
        *pos = prev;
    }
}

fn sentence_end(buffer: &Buffer, pos: (usize, usize)) -> Option<(usize, usize)> {
    let mut cursor = pos;
    loop {
        let (next, step) = inc_pos(buffer, cursor.0, cursor.1);
        if step == -1 {
            break;
        }
        cursor = next;
        if !matches!(
            char_at(buffer, cursor.0, cursor.1),
            Some(')' | ']' | '"' | '\'')
        ) {
            break;
        }
    }
    match char_at(buffer, cursor.0, cursor.1) {
        None => Some(cursor),
        Some(' ' | '\t') => Some(cursor),
        _ => None,
    }
}

fn is_empty_line(buffer: &Buffer, row: usize) -> bool {
    buffer.line_len(row) == 0
}

pub(crate) fn paragraph_target(
    buffer: &Buffer,
    row: usize,
    forward: bool,
    count: usize,
) -> Option<ParagraphTarget> {
    let last = buffer.line_count().saturating_sub(1);
    let mut curr = row;
    let mut remaining = count.max(1);
    let mut eof = false;
    while remaining > 0 {
        remaining -= 1;
        match paragraph_hop(buffer, curr, forward, remaining > 0, last) {
            Hop::Fail => return None,
            Hop::Target { line, edge } => {
                curr = line;
                eof = edge;
            }
        }
    }
    let col = if eof {
        buffer.line_len(curr).saturating_sub(1)
    } else {
        0
    };
    Some(ParagraphTarget {
        line: curr,
        col,
        eof,
    })
}

enum Hop {
    Target { line: usize, edge: bool },
    Fail,
}

fn paragraph_hop(buffer: &Buffer, row: usize, forward: bool, more: bool, last: usize) -> Hop {
    let mut curr = row;
    let mut did_skip = false;
    let mut first = true;
    loop {
        if buffer.line_len(curr) > 0 {
            did_skip = true;
        }
        if !first && did_skip && is_para_start(buffer, curr) {
            return Hop::Target {
                line: curr,
                edge: false,
            };
        }
        if forward {
            if curr >= last {
                if more {
                    return Hop::Fail;
                }
                return Hop::Target {
                    line: last,
                    edge: true,
                };
            }
            curr = curr.saturating_add(1);
        } else {
            if curr == 0 {
                if more {
                    return Hop::Fail;
                }
                return Hop::Target {
                    line: 0,
                    edge: false,
                };
            }
            curr = curr.saturating_sub(1);
        }
        first = false;
    }
}
