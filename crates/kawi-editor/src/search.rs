#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchOutcome {
    Found { wrapped: bool },
    Missing,
}

use kawi_buffer::Buffer;

pub fn find_first_from(
    buffer: &Buffer,
    pattern: &str,
    row: usize,
    col: usize,
) -> Option<(usize, usize)> {
    if pattern.is_empty() {
        return None;
    }
    let query = Query::new(pattern);
    buffer
        .line_slices()
        .enumerate()
        .skip(row)
        .find_map(|(r, slice)| {
            let bound = bound_forward_col(r, row, col);
            let text = slice_text(&slice);
            if !query.may_match(&text) {
                return None;
            }
            query.first_at_or_after(&text, bound).map(|c| (r, c))
        })
}

pub fn find_next(
    buffer: &Buffer,
    pattern: &str,
    row: usize,
    col: usize,
) -> Option<((usize, usize), bool)> {
    if pattern.is_empty() {
        return None;
    }
    let query = Query::new(pattern);
    if let Some(found) = scan_forward(
        buffer,
        &query,
        row,
        col.saturating_add(1),
        buffer.line_count(),
    ) {
        return Some((found, false));
    }
    scan_forward_wrapped(buffer, &query, row, col).map(|found| (found, true))
}

pub fn find_prev(
    buffer: &Buffer,
    pattern: &str,
    row: usize,
    col: usize,
) -> Option<((usize, usize), bool)> {
    if pattern.is_empty() {
        return None;
    }
    let query = Query::new(pattern);
    if let Some(found) = scan_backward(buffer, &query, row, col) {
        return Some((found, false));
    }
    scan_backward_wrapped(buffer, &query, row).map(|found| (found, true))
}

fn scan_forward(
    buffer: &Buffer,
    query: &Query,
    row: usize,
    col: usize,
    end_row: usize,
) -> Option<(usize, usize)> {
    buffer
        .line_slices()
        .enumerate()
        .skip(row)
        .take(end_row.saturating_sub(row))
        .find_map(|(r, slice)| {
            let bound = bound_forward_col(r, row, col);
            let text = slice_text(&slice);
            if !query.may_match(&text) {
                return None;
            }
            query.first_at_or_after(&text, bound).map(|c| (r, c))
        })
}

fn bound_forward_col(line_row: usize, origin_row: usize, origin_col: usize) -> usize {
    if line_row == origin_row {
        return origin_col;
    }
    0
}

fn scan_forward_wrapped(
    buffer: &Buffer,
    query: &Query,
    row: usize,
    col: usize,
) -> Option<(usize, usize)> {
    if let Some(found) = scan_forward(buffer, query, 0, 0, row) {
        return Some(found);
    }
    buffer
        .line_cow(row)
        .and_then(|line| query.first_before(&line, col).map(|c| (row, c)))
}

fn slice_text<'a>(slice: &'a ropey::RopeSlice<'a>) -> std::borrow::Cow<'a, str> {
    match slice.as_str() {
        Some(text) => std::borrow::Cow::Borrowed(text),
        None => std::borrow::Cow::Owned(slice.to_string()),
    }
}

fn scan_backward(buffer: &Buffer, query: &Query, row: usize, col: usize) -> Option<(usize, usize)> {
    let mut found = None;
    for (r, slice) in buffer.line_slices().enumerate().take(row.saturating_add(1)) {
        let text = slice_text(&slice);
        if !query.may_match(&text) {
            continue;
        }
        let hit = scan_backward_row(&text, query, r, row, col);
        if hit.is_some() {
            found = hit;
        }
    }
    found
}

fn scan_backward_row(
    text: &str,
    query: &Query,
    line_row: usize,
    origin_row: usize,
    origin_col: usize,
) -> Option<(usize, usize)> {
    if line_row == origin_row {
        query.last_before(text, origin_col).map(|c| (line_row, c))
    } else {
        query.last_in_line(text).map(|c| (line_row, c))
    }
}

fn scan_backward_wrapped(buffer: &Buffer, query: &Query, row: usize) -> Option<(usize, usize)> {
    let mut found = None;
    for (r, slice) in buffer.line_slices().enumerate().skip(row.saturating_add(1)) {
        let text = slice_text(&slice);
        if !query.may_match(&text) {
            continue;
        }
        if let Some(c) = query.last_in_line(&text) {
            found = Some((r, c));
        }
    }
    if found.is_some() {
        return found;
    }
    buffer
        .line_cow(row)
        .and_then(|line| query.last_in_line(&line).map(|c| (row, c)))
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

    fn may_match(&self, line: &str) -> bool {
        let first = match self.text.chars().next() {
            Some(ch) => ch,
            None => return false,
        };
        if self.sensitive {
            return line.contains(first);
        }
        if first.is_ascii_alphabetic() {
            return line.contains(first.to_ascii_lowercase())
                || line.contains(first.to_ascii_uppercase());
        }
        if first.is_ascii() {
            return line.contains(first);
        }
        true
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

    fn lines_of(text: &str) -> Buffer {
        Buffer::from_string(text)
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
