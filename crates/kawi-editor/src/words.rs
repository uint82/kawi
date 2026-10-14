use kawi_buffer::Buffer;

#[derive(Clone, Copy, PartialEq, Eq)]
enum WordClass {
    Word,
    Punct,
    Space,
}

pub fn next_word_start(buffer: &Buffer, row: usize, col: usize) -> (usize, usize) {
    let mut pos = next_slot(buffer, row, col);
    while let Some((r, c)) = pos {
        if line_len(buffer, r) == 0 {
            return (r, c);
        }
        if let Some(found) = word_start_at(buffer, r, c) {
            return found;
        }
        pos = next_slot(buffer, r, c);
    }
    (row, col)
}

pub fn prev_word_start(buffer: &Buffer, row: usize, col: usize) -> (usize, usize) {
    let mut pos = prev_slot(buffer, row, col);
    while let Some((r, c)) = pos {
        if line_len(buffer, r) == 0 {
            return (r, c);
        }
        if let Some(found) = word_start_at(buffer, r, c) {
            return found;
        }
        pos = prev_slot(buffer, r, c);
    }
    (row, col)
}

pub fn next_word_end(buffer: &Buffer, row: usize, col: usize) -> (usize, usize) {
    let mut pos = next_slot(buffer, row, col);
    while let Some((r, c)) = pos {
        if let Some(found) = word_end_at(buffer, r, c) {
            return found;
        }
        pos = next_slot(buffer, r, c);
    }
    (row, col)
}

pub fn dw_range(
    buffer: &Buffer,
    row: usize,
    col: usize,
) -> Option<((usize, usize), (usize, usize))> {
    let target = next_word_start(buffer, row, col);
    let end_exclusive = dw_end(buffer, row, col, target);
    let (end_row, end_col) = end_exclusive;
    let end = prev_char_pos(buffer, end_row, end_col)?;
    if (row, col) == end_exclusive {
        return None;
    }
    Some(((row, col), end))
}

fn dw_end(buffer: &Buffer, row: usize, col: usize, target: (usize, usize)) -> (usize, usize) {
    if target != (row, col) {
        return target;
    }
    (row, line_len(buffer, row))
}

pub fn cw_range(
    buffer: &Buffer,
    row: usize,
    col: usize,
) -> Option<((usize, usize), (usize, usize))> {
    if line_len(buffer, row) == 0 {
        return None;
    }
    Some(((row, col), next_word_end(buffer, row, col)))
}

fn word_start_at(buffer: &Buffer, row: usize, col: usize) -> Option<(usize, usize)> {
    let ch = char_at(buffer, row, col)?;
    if class_of(ch) == WordClass::Space {
        return None;
    }
    if is_word_start(buffer, row, col) {
        return Some((row, col));
    }
    None
}

fn is_word_start(buffer: &Buffer, row: usize, col: usize) -> bool {
    if col == 0 {
        return true;
    }
    match char_at(buffer, row, col.saturating_sub(1)) {
        None => true,
        Some(prev) => {
            let current = char_at(buffer, row, col).map(class_of);
            let previous = class_of(prev);
            previous == WordClass::Space || current != Some(previous)
        }
    }
}

fn word_end_at(buffer: &Buffer, row: usize, col: usize) -> Option<(usize, usize)> {
    let ch = char_at(buffer, row, col)?;
    if class_of(ch) == WordClass::Space {
        return None;
    }
    Some(run_end(buffer, row, col, class_of(ch)))
}

fn run_end(buffer: &Buffer, row: usize, col: usize, class: WordClass) -> (usize, usize) {
    let mut end = (row, col);
    let mut pos = next_slot(buffer, row, col);
    while let Some((r, c)) = pos {
        match char_at(buffer, r, c) {
            Some(ch) if class_of(ch) == class => end = (r, c),
            _ => break,
        }
        pos = next_slot(buffer, r, c);
    }
    end
}

fn next_slot(buffer: &Buffer, row: usize, col: usize) -> Option<(usize, usize)> {
    if col.saturating_add(1) < line_len(buffer, row) {
        return Some((row, col.saturating_add(1)));
    }
    if row.saturating_add(1) < buffer.line_count() {
        return Some((row.saturating_add(1), 0));
    }
    None
}

fn prev_slot(buffer: &Buffer, row: usize, col: usize) -> Option<(usize, usize)> {
    if col > 0 {
        return Some((row, col.saturating_sub(1)));
    }
    if row > 0 {
        let prev = row.saturating_sub(1);
        let len = line_len(buffer, prev);
        if len == 0 {
            return Some((prev, 0));
        }
        return Some((prev, len.saturating_sub(1)));
    }
    None
}

fn prev_char_pos(buffer: &Buffer, row: usize, col: usize) -> Option<(usize, usize)> {
    if col > 0 {
        return char_at(buffer, row, col.saturating_sub(1)).map(|_| (row, col.saturating_sub(1)));
    }
    if row == 0 {
        return None;
    }
    let prev = row.saturating_sub(1);
    backward_char_pos(buffer, prev)
}

fn backward_char_pos(buffer: &Buffer, mut row: usize) -> Option<(usize, usize)> {
    loop {
        let len = line_len(buffer, row);
        if len > 0 {
            return Some((row, len.saturating_sub(1)));
        }
        if row == 0 {
            return None;
        }
        row = row.saturating_sub(1);
    }
}

fn char_at(buffer: &Buffer, row: usize, col: usize) -> Option<char> {
    buffer.line_cow(row)?.chars().nth(col)
}

fn line_len(buffer: &Buffer, row: usize) -> usize {
    buffer.line_len(row)
}

fn class_of(ch: char) -> WordClass {
    if ch.is_alphanumeric() || ch == '_' {
        WordClass::Word
    } else if ch.is_whitespace() {
        WordClass::Space
    } else {
        WordClass::Punct
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_buffer::Buffer;

    fn lines_of(text: &str) -> Buffer {
        Buffer::from_string(text)
    }

    #[test]
    fn w_moves_to_next_start() {
        let lines = lines_of("foo bar");
        assert_eq!(next_word_start(&lines, 0, 0), (0, 4));
    }

    #[test]
    fn w_stops_on_punct() {
        let lines = lines_of("foo, bar");
        assert_eq!(next_word_start(&lines, 0, 0), (0, 3));
        assert_eq!(next_word_start(&lines, 0, 3), (0, 5));
    }

    #[test]
    fn w_skips_gaps() {
        let lines = lines_of("foo   bar");
        assert_eq!(next_word_start(&lines, 0, 1), (0, 6));
    }

    #[test]
    fn w_stays_at_last_word() {
        let lines = lines_of("foo");
        assert_eq!(next_word_start(&lines, 0, 2), (0, 2));
    }

    #[test]
    fn w_stops_on_empty_line() {
        let lines = lines_of("foo\n\nbar");
        assert_eq!(next_word_start(&lines, 0, 0), (1, 0));
        assert_eq!(next_word_start(&lines, 1, 0), (2, 0));
    }

    #[test]
    fn b_moves_to_prev_start() {
        let lines = lines_of("foo bar");
        assert_eq!(prev_word_start(&lines, 0, 4), (0, 0));
    }

    #[test]
    fn b_stays_at_first_word() {
        let lines = lines_of("foo");
        assert_eq!(prev_word_start(&lines, 0, 0), (0, 0));
    }

    #[test]
    fn b_skips_leading_space() {
        let lines = lines_of("  foo");
        assert_eq!(prev_word_start(&lines, 0, 2), (0, 2));
    }

    #[test]
    fn b_stops_on_empty_line() {
        let lines = lines_of("foo\n\nbar");
        assert_eq!(prev_word_start(&lines, 2, 0), (1, 0));
        assert_eq!(prev_word_start(&lines, 1, 0), (0, 0));
    }

    #[test]
    fn b_crosses_lines() {
        let lines = lines_of("foo\nbar");
        assert_eq!(prev_word_start(&lines, 1, 0), (0, 0));
    }

    #[test]
    fn e_moves_to_word_end() {
        let lines = lines_of("foo bar");
        assert_eq!(next_word_end(&lines, 0, 0), (0, 2));
        assert_eq!(next_word_end(&lines, 0, 2), (0, 6));
    }

    #[test]
    fn e_stops_on_punct_end() {
        let lines = lines_of("foo, bar");
        assert_eq!(next_word_end(&lines, 0, 2), (0, 3));
        assert_eq!(next_word_end(&lines, 0, 3), (0, 7));
    }

    #[test]
    fn e_skips_empty_lines() {
        let lines = lines_of("foo\n\nbar");
        assert_eq!(next_word_end(&lines, 0, 2), (2, 2));
    }

    #[test]
    fn e_stays_at_last_end() {
        let lines = lines_of("foo");
        assert_eq!(next_word_end(&lines, 0, 2), (0, 2));
    }

    #[test]
    fn dw_deletes_to_next_start() {
        let lines = lines_of("foo bar");
        assert_eq!(dw_range(&lines, 0, 0), Some(((0, 0), (0, 3))));
    }

    #[test]
    fn dw_last_word_deletes_to_end() {
        let lines = lines_of("foo bar");
        assert_eq!(dw_range(&lines, 0, 4), Some(((0, 4), (0, 6))));
    }

    #[test]
    fn dw_space_deletes_spaces() {
        let lines = lines_of("foo   bar");
        assert_eq!(dw_range(&lines, 0, 3), Some(((0, 3), (0, 5))));
    }

    #[test]
    fn dw_last_char_deletes_single() {
        let lines = lines_of("foo");
        assert_eq!(dw_range(&lines, 0, 2), Some(((0, 2), (0, 2))));
    }

    #[test]
    fn dw_empty_line_is_none() {
        let lines = lines_of("");
        assert_eq!(dw_range(&lines, 0, 0), None);
    }

    #[test]
    fn dw_across_lines_stops_before_target() {
        let lines = lines_of("foo\nbar");
        assert_eq!(dw_range(&lines, 0, 0), Some(((0, 0), (0, 2))));
    }

    #[test]
    fn dw_empty_line_keeps_newline() {
        let lines = lines_of("foo\n\nbar");
        assert_eq!(dw_range(&lines, 0, 0), Some(((0, 0), (0, 2))));
    }

    #[test]
    fn cw_deletes_to_word_end() {
        let lines = lines_of("foo bar");
        assert_eq!(cw_range(&lines, 0, 0), Some(((0, 0), (0, 2))));
        assert_eq!(cw_range(&lines, 0, 1), Some(((0, 1), (0, 2))));
    }

    #[test]
    fn cw_empty_line_is_none() {
        let lines = lines_of("");
        assert_eq!(cw_range(&lines, 0, 0), None);
    }
}
