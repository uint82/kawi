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
    let query = Query::new(pattern);
    lines
        .iter()
        .enumerate()
        .skip(row)
        .filter_map(|(r, line)| {
            let bound = bound_forward_col(r, row, col);
            query.first_at_or_after(line, bound).map(|c| (r, c))
        })
        .next()
}

pub fn find_next(
    lines: &[String],
    pattern: &str,
    row: usize,
    col: usize,
) -> Option<((usize, usize), bool)> {
    if pattern.is_empty() {
        return None;
    }
    let query = Query::new(pattern);
    if let Some(found) = scan_forward(lines, &query, row, col.saturating_add(1), lines.len()) {
        return Some((found, false));
    }
    scan_forward_wrapped(lines, &query, row, col).map(|found| (found, true))
}

pub fn find_prev(
    lines: &[String],
    pattern: &str,
    row: usize,
    col: usize,
) -> Option<((usize, usize), bool)> {
    if pattern.is_empty() {
        return None;
    }
    let query = Query::new(pattern);
    if let Some(found) = scan_backward(lines, &query, row, col) {
        return Some((found, false));
    }
    scan_backward_wrapped(lines, &query, row).map(|found| (found, true))
}

fn scan_forward(
    lines: &[String],
    query: &Query,
    row: usize,
    col: usize,
    end_row: usize,
) -> Option<(usize, usize)> {
    lines
        .iter()
        .enumerate()
        .skip(row)
        .take(end_row.saturating_sub(row))
        .filter_map(|(r, line)| {
            let bound = bound_forward_col(r, row, col);
            query.first_at_or_after(line, bound).map(|c| (r, c))
        })
        .next()
}

fn bound_forward_col(line_row: usize, origin_row: usize, origin_col: usize) -> usize {
    if line_row == origin_row {
        return origin_col;
    }
    0
}

fn scan_forward_wrapped(
    lines: &[String],
    query: &Query,
    row: usize,
    col: usize,
) -> Option<(usize, usize)> {
    if let Some(found) = scan_forward(lines, query, 0, 0, row) {
        return Some(found);
    }
    lines
        .get(row)
        .and_then(|line| query.first_before(line, col).map(|c| (row, c)))
}

fn scan_backward(
    lines: &[String],
    query: &Query,
    row: usize,
    col: usize,
) -> Option<(usize, usize)> {
    lines
        .iter()
        .enumerate()
        .take(row.saturating_add(1))
        .rev()
        .filter_map(|(r, line)| {
            if r == row {
                query.last_before(line, col).map(|c| (r, c))
            } else {
                query.last_in_line(line).map(|c| (r, c))
            }
        })
        .next()
}

fn scan_backward_wrapped(lines: &[String], query: &Query, row: usize) -> Option<(usize, usize)> {
    if let Some(found) = lines
        .iter()
        .enumerate()
        .skip(row.saturating_add(1))
        .rev()
        .filter_map(|(r, line)| query.last_in_line(line).map(|c| (r, c)))
        .next()
    {
        return Some(found);
    }
    lines
        .get(row)
        .and_then(|line| query.last_in_line(line).map(|c| (row, c)))
}

fn byte_of_col(line: &str, col: usize) -> usize {
    line.char_indices()
        .nth(col)
        .map_or(line.len(), |(byte, _)| byte)
}

fn col_of_byte(line: &str, byte: usize) -> usize {
    line[..byte.min(line.len())].chars().count()
}

fn pattern_has_upper(pattern: &str) -> bool {
    pattern.chars().any(|ch| ch.is_uppercase())
}

fn fold_case(line: &str) -> std::borrow::Cow<'_, str> {
    if line.is_ascii() {
        return fold_ascii(line);
    }
    std::borrow::Cow::Owned(line.to_lowercase())
}

fn fold_ascii(line: &str) -> std::borrow::Cow<'_, str> {
    if line.bytes().any(|byte| byte.is_ascii_uppercase()) {
        return std::borrow::Cow::Owned(line.to_ascii_lowercase());
    }
    std::borrow::Cow::Borrowed(line)
}

struct Query {
    text: String,
    sensitive: bool,
}

impl Query {
    fn new(pattern: &str) -> Self {
        if pattern_has_upper(pattern) {
            Self {
                text: pattern.to_string(),
                sensitive: true,
            }
        } else {
            Self {
                text: pattern.to_lowercase(),
                sensitive: false,
            }
        }
    }

    fn haystack<'a>(&self, line: &'a str) -> std::borrow::Cow<'a, str> {
        if self.sensitive {
            return std::borrow::Cow::Borrowed(line);
        }
        fold_case(line)
    }

    fn first_at_or_after(&self, line: &str, bound_col: usize) -> Option<usize> {
        let hay = self.haystack(line);
        let bound_byte = byte_of_col(&hay, bound_col);
        hay.match_indices(&self.text)
            .map(|(byte, _)| byte)
            .find(|byte| *byte >= bound_byte)
            .map(|byte| col_of_byte(&hay, byte))
    }

    fn first_before(&self, line: &str, bound_col: usize) -> Option<usize> {
        let bound_byte = byte_of_col(line, bound_col);
        let hay = self.haystack(line);
        hay.match_indices(&self.text)
            .map(|(byte, _)| byte)
            .take_while(|byte| *byte < bound_byte)
            .next()
            .map(|byte| col_of_byte(&hay, byte))
    }

    fn last_before(&self, line: &str, bound_col: usize) -> Option<usize> {
        let hay = self.haystack(line);
        let bound_byte = byte_of_col(&hay, bound_col);
        hay.match_indices(&self.text)
            .map(|(byte, _)| byte)
            .take_while(|byte| *byte < bound_byte)
            .last()
            .map(|byte| col_of_byte(&hay, byte))
    }

    fn last_in_line(&self, line: &str) -> Option<usize> {
        let hay = self.haystack(line);
        hay.match_indices(&self.text)
            .map(|(byte, _)| byte)
            .last()
            .map(|byte| col_of_byte(&hay, byte))
    }
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
    fn folds_non_ascii_case() {
        let lines = lines_of("CAFÉ x");
        assert_eq!(find_first_from(&lines, "café", 0, 0), Some((0, 0)));
    }

    #[test]
    fn finds_across_lines() {
        let lines = lines_of("foo\nbar");
        assert_eq!(find_first_from(&lines, "ba", 0, 0), Some((1, 0)));
        assert_eq!(find_prev(&lines, "fo", 1, 2), Some(((0, 0), false)));
    }
}
