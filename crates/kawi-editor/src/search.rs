#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchOutcome {
    Found { wrapped: bool },
    Missing,
}

pub fn find_first_from(
    lines: &[String],
    pattern: &str,
    row: usize,
    col: usize,
) -> Option<(usize, usize)> {
    if pattern.is_empty() {
        return None;
    }
    let mut pos = Some((row, col));
    while let Some((r, c)) = pos {
        if matches_at(lines, r, c, pattern) {
            return Some((r, c));
        }
        pos = next_char_pos(lines, r, c);
    }
    None
}

pub fn find_next(
    lines: &[String],
    pattern: &str,
    row: usize,
    col: usize,
) -> Option<((usize, usize), bool)> {
    find_from(lines, pattern, row, col, true)
}

pub fn find_prev(
    lines: &[String],
    pattern: &str,
    row: usize,
    col: usize,
) -> Option<((usize, usize), bool)> {
    find_from(lines, pattern, row, col, false)
}

fn find_from(
    lines: &[String],
    pattern: &str,
    row: usize,
    col: usize,
    forward: bool,
) -> Option<((usize, usize), bool)> {
    if pattern.is_empty() {
        return None;
    }
    let start = step_over(lines, row, col, forward);
    if let Some(origin) = start {
        if let Some(found) = scan(lines, pattern, origin, forward) {
            return Some((found, false));
        }
    }
    let begin = restart_pos(lines, forward)?;
    scan_until(lines, pattern, begin, start, forward).map(|found| (found, true))
}

fn scan_until(
    lines: &[String],
    pattern: &str,
    begin: (usize, usize),
    stop: Option<(usize, usize)>,
    forward: bool,
) -> Option<(usize, usize)> {
    let mut pos = Some(begin);
    while let Some((r, c)) = pos {
        if Some((r, c)) == stop {
            break;
        }
        if matches_at(lines, r, c, pattern) {
            return Some((r, c));
        }
        pos = step(lines, r, c, forward);
    }
    None
}

fn step_over(lines: &[String], row: usize, col: usize, forward: bool) -> Option<(usize, usize)> {
    if forward {
        step_forward(lines, row, col)
    } else {
        step_backward(lines, row, col)
    }
}

fn scan(
    lines: &[String],
    pattern: &str,
    start: (usize, usize),
    forward: bool,
) -> Option<(usize, usize)> {
    let mut pos = Some(start);
    while let Some((r, c)) = pos {
        if matches_at(lines, r, c, pattern) {
            return Some((r, c));
        }
        pos = step(lines, r, c, forward);
    }
    None
}

fn restart_pos(lines: &[String], forward: bool) -> Option<(usize, usize)> {
    if forward {
        first_char_pos(lines)
    } else {
        last_char_pos(lines)
    }
}

fn step(lines: &[String], row: usize, col: usize, forward: bool) -> Option<(usize, usize)> {
    if forward {
        step_forward(lines, row, col)
    } else {
        step_backward(lines, row, col)
    }
}

fn step_forward(lines: &[String], row: usize, col: usize) -> Option<(usize, usize)> {
    if col.saturating_add(1) < line_len(lines, row) {
        return Some((row, col.saturating_add(1)));
    }
    if row.saturating_add(1) < lines.len() {
        return Some((row.saturating_add(1), 0));
    }
    None
}

fn step_backward(lines: &[String], row: usize, col: usize) -> Option<(usize, usize)> {
    if col > 0 {
        return char_at(lines, row, col.saturating_sub(1)).map(|_| (row, col.saturating_sub(1)));
    }
    if row == 0 {
        return None;
    }
    backward_char_pos(lines, row.saturating_sub(1))
}

fn backward_char_pos(lines: &[String], mut row: usize) -> Option<(usize, usize)> {
    loop {
        let len = line_len(lines, row);
        if len > 0 {
            return Some((row, len.saturating_sub(1)));
        }
        if row == 0 {
            return None;
        }
        row = row.saturating_sub(1);
    }
}

fn first_char_pos(lines: &[String]) -> Option<(usize, usize)> {
    (0..lines.len()).find_map(|row| char_at(lines, row, 0).map(|_| (row, 0)))
}

fn last_char_pos(lines: &[String]) -> Option<(usize, usize)> {
    (0..lines.len()).rev().find_map(|row| {
        let len = line_len(lines, row);
        if len > 0 {
            return Some((row, len.saturating_sub(1)));
        }
        None
    })
}

fn next_char_pos(lines: &[String], row: usize, col: usize) -> Option<(usize, usize)> {
    step_forward(lines, row, col)
}

fn matches_at(lines: &[String], row: usize, col: usize, pattern: &str) -> bool {
    let line = match lines.get(row) {
        Some(line) => line,
        None => return false,
    };
    let chars: Vec<char> = line.chars().collect();
    let wanted: Vec<char> = pattern.chars().collect();
    if col.saturating_add(wanted.len()) > chars.len() {
        return false;
    }
    let sensitive = pattern_has_upper(pattern);
    chars
        .iter()
        .skip(col)
        .zip(wanted.iter())
        .all(|(found, want)| chars_match(*found, *want, sensitive))
}

fn pattern_has_upper(pattern: &str) -> bool {
    pattern.chars().any(|ch| ch.is_uppercase())
}

fn chars_match(found: char, wanted: char, sensitive: bool) -> bool {
    if sensitive {
        return found == wanted;
    }
    lower_char(found) == lower_char(wanted)
}

fn lower_char(ch: char) -> char {
    ch.to_lowercase().next().unwrap_or(ch)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn lines_of(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
    }

    #[test]
    fn finds_first_from_origin() {
        let lines = lines_of("foo bar foo");
        assert_eq!(find_first_from(&lines, "ar", 0, 0), Some((0, 5)));
    }

    #[test]
    fn empty_pattern_finds_nothing() {
        let lines = lines_of("foo");
        assert_eq!(find_first_from(&lines, "", 0, 0), None);
        assert_eq!(find_next(&lines, "", 0, 0), None);
        assert_eq!(find_prev(&lines, "", 0, 0), None);
    }

    #[test]
    fn next_steps_through_matches() {
        let lines = lines_of("ar ar ar");
        assert_eq!(find_next(&lines, "ar", 0, 0), Some(((0, 3), false)));
        assert_eq!(find_next(&lines, "ar", 0, 3), Some(((0, 6), false)));
    }

    #[test]
    fn next_wraps_to_first() {
        let lines = lines_of("ar ar ar");
        assert_eq!(find_next(&lines, "ar", 0, 6), Some(((0, 0), true)));
    }

    #[test]
    fn prev_steps_back_and_wraps() {
        let lines = lines_of("ar ar ar");
        assert_eq!(find_prev(&lines, "ar", 0, 6), Some(((0, 3), false)));
        assert_eq!(find_prev(&lines, "ar", 0, 0), Some(((0, 6), true)));
    }

    #[test]
    fn no_match_returns_none() {
        let lines = lines_of("foo");
        assert_eq!(find_next(&lines, "zzz", 0, 0), None);
        assert_eq!(find_first_from(&lines, "zzz", 0, 0), None);
    }

    #[test]
    fn uppercase_pattern_is_sensitive() {
        let lines = lines_of("AR ar");
        assert_eq!(find_first_from(&lines, "AR", 0, 0), Some((0, 0)));
    }

    #[test]
    fn lowercase_pattern_ignores_case() {
        let lines = lines_of("AR ar");
        assert_eq!(find_first_from(&lines, "ar", 0, 0), Some((0, 0)));
        assert_eq!(find_next(&lines, "ar", 0, 0), Some(((0, 3), false)));
    }

    #[test]
    fn finds_across_lines() {
        let lines = lines_of("foo\nbar");
        assert_eq!(find_first_from(&lines, "ba", 0, 0), Some((1, 0)));
        assert_eq!(find_prev(&lines, "fo", 1, 2), Some(((0, 0), false)));
    }
}
