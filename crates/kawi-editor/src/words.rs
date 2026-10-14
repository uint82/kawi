#[derive(Clone, Copy, PartialEq, Eq)]
enum WordClass {
    Word,
    Punct,
    Space,
}

pub fn next_word_start(lines: &[String], row: usize, col: usize) -> (usize, usize) {
    let mut pos = next_slot(lines, row, col);
    while let Some((r, c)) = pos {
        if line_len(lines, r) == 0 {
            return (r, c);
        }
        if let Some(found) = word_start_at(lines, r, c) {
            return found;
        }
        pos = next_slot(lines, r, c);
    }
    (row, col)
}

pub fn prev_word_start(lines: &[String], row: usize, col: usize) -> (usize, usize) {
    let mut pos = prev_slot(lines, row, col);
    while let Some((r, c)) = pos {
        if line_len(lines, r) == 0 {
            return (r, c);
        }
        if let Some(found) = word_start_at(lines, r, c) {
            return found;
        }
        pos = prev_slot(lines, r, c);
    }
    (row, col)
}

pub fn next_word_end(lines: &[String], row: usize, col: usize) -> (usize, usize) {
    let mut pos = next_slot(lines, row, col);
    while let Some((r, c)) = pos {
        if let Some(found) = word_end_at(lines, r, c) {
            return found;
        }
        pos = next_slot(lines, r, c);
    }
    (row, col)
}

fn word_start_at(lines: &[String], row: usize, col: usize) -> Option<(usize, usize)> {
    let ch = char_at(lines, row, col)?;
    if class_of(ch) == WordClass::Space {
        return None;
    }
    if is_word_start(lines, row, col) {
        return Some((row, col));
    }
    None
}

fn is_word_start(lines: &[String], row: usize, col: usize) -> bool {
    if col == 0 {
        return true;
    }
    match char_at(lines, row, col.saturating_sub(1)) {
        None => true,
        Some(prev) => {
            let current = char_at(lines, row, col).map(class_of);
            let previous = class_of(prev);
            previous == WordClass::Space || current != Some(previous)
        }
    }
}

fn word_end_at(lines: &[String], row: usize, col: usize) -> Option<(usize, usize)> {
    let ch = char_at(lines, row, col)?;
    if class_of(ch) == WordClass::Space {
        return None;
    }
    Some(run_end(lines, row, col, class_of(ch)))
}

fn run_end(lines: &[String], row: usize, col: usize, class: WordClass) -> (usize, usize) {
    let mut end = (row, col);
    let mut pos = next_slot(lines, row, col);
    while let Some((r, c)) = pos {
        match char_at(lines, r, c) {
            Some(ch) if class_of(ch) == class => end = (r, c),
            _ => break,
        }
        pos = next_slot(lines, r, c);
    }
    end
}

fn next_slot(lines: &[String], row: usize, col: usize) -> Option<(usize, usize)> {
    if col.saturating_add(1) < line_len(lines, row) {
        return Some((row, col.saturating_add(1)));
    }
    if row.saturating_add(1) < lines.len() {
        return Some((row.saturating_add(1), 0));
    }
    None
}

fn prev_slot(lines: &[String], row: usize, col: usize) -> Option<(usize, usize)> {
    if col > 0 {
        return Some((row, col.saturating_sub(1)));
    }
    if row > 0 {
        let prev = row.saturating_sub(1);
        let len = line_len(lines, prev);
        if len == 0 {
            return Some((prev, 0));
        }
        return Some((prev, len.saturating_sub(1)));
    }
    None
}

fn char_at(lines: &[String], row: usize, col: usize) -> Option<char> {
    lines.get(row)?.chars().nth(col)
}

fn line_len(lines: &[String], row: usize) -> usize {
    lines.get(row).map_or(0, |line| char_count(line))
}

fn char_count(text: &str) -> usize {
    text.chars().count()
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

    fn lines_of(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
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
}
