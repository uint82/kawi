use kawi_buffer::Buffer;
use ropey::RopeSlice;

use crate::matcher::{self, Query};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchOutcome {
    Found { wrapped: bool },
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchSpan {
    pub row: usize,
    pub start: usize,
    pub end: usize,
    pub current: bool,
}

pub fn collect_matches(
    buffer: &Buffer,
    pattern: &str,
    start_row: usize,
    end_row: usize,
) -> Vec<MatchSpan> {
    let query = Query::new(pattern);
    if query.is_empty() {
        return Vec::new();
    }
    if query.needs_folded_lines() {
        return lines_collect_matches(buffer, &query, start_row, end_row);
    }
    if start_row >= end_row {
        return Vec::new();
    }
    let slice = buffer.rope_slice();
    let from = position_byte(&slice, buffer, start_row, 0);
    let end = end_row_byte(&slice, buffer, end_row);
    matcher::spans_in_range(&slice, &query, from, end)
        .into_iter()
        .map(|(start_byte, end_byte)| span_of_bytes(&slice, buffer, start_byte, end_byte))
        .collect()
}

fn end_row_byte(slice: &RopeSlice, buffer: &Buffer, end_row: usize) -> usize {
    if end_row >= buffer.line_count() {
        return slice.len_bytes();
    }
    position_byte(slice, buffer, end_row, 0)
}

fn span_of_bytes(
    slice: &RopeSlice,
    buffer: &Buffer,
    start_byte: usize,
    end_byte: usize,
) -> MatchSpan {
    let (row, start) = position_of_byte(slice, buffer, start_byte);
    let (end_row, end) = position_of_byte(slice, buffer, end_byte);
    let end = span_end_col(buffer, row, end_row, end);
    MatchSpan {
        row,
        start,
        end,
        current: false,
    }
}

fn span_end_col(buffer: &Buffer, row: usize, end_row: usize, end: usize) -> usize {
    if end_row == row {
        return end.min(buffer.line_len(row));
    }
    buffer.line_len(row)
}

pub fn find_first_from(
    buffer: &Buffer,
    pattern: &str,
    row: usize,
    col: usize,
) -> Option<(usize, usize)> {
    let query = Query::new(pattern);
    if query.is_empty() {
        return None;
    }
    if query.needs_folded_lines() {
        return lines_first_from(buffer, &query, row, col);
    }
    let slice = buffer.rope_slice();
    let from = position_byte(&slice, buffer, row, col);
    matcher::first_in_range(&slice, &query, from, slice.len_bytes())
        .map(|byte| position_of_byte(&slice, buffer, byte))
}

pub fn find_next(
    buffer: &Buffer,
    pattern: &str,
    row: usize,
    col: usize,
) -> Option<((usize, usize), bool)> {
    let query = Query::new(pattern);
    if query.is_empty() {
        return None;
    }
    if query.needs_folded_lines() {
        return lines_find_next(buffer, &query, row, col);
    }
    let slice = buffer.rope_slice();
    let from = position_byte(&slice, buffer, row, col.saturating_add(1));
    if let Some(byte) = matcher::first_in_range(&slice, &query, from, slice.len_bytes()) {
        return Some((position_of_byte(&slice, buffer, byte), false));
    }
    wrapped_first(&slice, buffer, &query, row, col).map(|found| (found, true))
}

pub fn find_prev(
    buffer: &Buffer,
    pattern: &str,
    row: usize,
    col: usize,
) -> Option<((usize, usize), bool)> {
    let query = Query::new(pattern);
    if query.is_empty() {
        return None;
    }
    if query.needs_folded_lines() {
        return lines_find_prev(buffer, &query, row, col);
    }
    let slice = buffer.rope_slice();
    let bound = position_byte(&slice, buffer, row, col);
    if let Some(byte) = matcher::last_in_range(&slice, &query, 0, bound) {
        return Some((position_of_byte(&slice, buffer, byte), false));
    }
    wrapped_last(&slice, buffer, &query, row).map(|found| (found, true))
}

fn wrapped_first(
    slice: &RopeSlice,
    buffer: &Buffer,
    query: &Query,
    row: usize,
    col: usize,
) -> Option<(usize, usize)> {
    let start = row_start_byte(slice, buffer, row);
    if let Some(byte) = matcher::first_in_range(slice, query, 0, start) {
        return Some(position_of_byte(slice, buffer, byte));
    }
    let bound = position_byte(slice, buffer, row, col);
    matcher::first_in_range(slice, query, start, bound)
        .map(|byte| position_of_byte(slice, buffer, byte))
}

fn wrapped_last(
    slice: &RopeSlice,
    buffer: &Buffer,
    query: &Query,
    row: usize,
) -> Option<(usize, usize)> {
    let next = next_row_start_byte(slice, buffer, row);
    if let Some(byte) = matcher::last_in_range(slice, query, next, slice.len_bytes()) {
        return Some(position_of_byte(slice, buffer, byte));
    }
    let start = row_start_byte(slice, buffer, row);
    matcher::last_in_range(slice, query, start, next)
        .map(|byte| position_of_byte(slice, buffer, byte))
}

pub fn count_matches(buffer: &Buffer, pattern: &str) -> usize {
    let query = Query::new(pattern);
    if query.is_empty() {
        return 0;
    }
    if query.needs_folded_lines() {
        return lines_count_matches(buffer, &query);
    }
    let slice = buffer.rope_slice();
    matcher::count_in_range(&slice, &query, 0, slice.len_bytes())
}

pub fn match_rank(buffer: &Buffer, pattern: &str, row: usize, col: usize, total: usize) -> usize {
    let query = Query::new(pattern);
    if query.is_empty() || total == 0 {
        return 0;
    }
    let before = matches_before(buffer, &query, row, col);
    before.saturating_add(1).min(total)
}

fn matches_before(buffer: &Buffer, query: &Query, row: usize, col: usize) -> usize {
    if query.needs_folded_lines() {
        return lines_matches_before(buffer, query, row, col);
    }
    let slice = buffer.rope_slice();
    matcher::count_in_range(&slice, query, 0, position_byte(&slice, buffer, row, col))
}

fn position_byte(slice: &RopeSlice, buffer: &Buffer, row: usize, col: usize) -> usize {
    let total = buffer.total_chars();
    slice.char_to_byte(buffer.char_index_of(row, col).min(total))
}

fn position_of_byte(slice: &RopeSlice, buffer: &Buffer, byte: usize) -> (usize, usize) {
    buffer.position_of_char(slice.byte_to_char(byte.min(slice.len_bytes())))
}

fn row_start_byte(slice: &RopeSlice, buffer: &Buffer, row: usize) -> usize {
    position_byte(slice, buffer, row, 0)
}

fn next_row_start_byte(slice: &RopeSlice, buffer: &Buffer, row: usize) -> usize {
    if row.saturating_add(1) >= buffer.line_count() {
        return slice.len_bytes();
    }
    row_start_byte(slice, buffer, row.saturating_add(1))
}

fn lines_find_next(
    buffer: &Buffer,
    query: &Query,
    row: usize,
    col: usize,
) -> Option<((usize, usize), bool)> {
    if let Some(found) = lines_scan_forward(
        buffer,
        query,
        row,
        col.saturating_add(1),
        buffer.line_count(),
    ) {
        return Some((found, false));
    }
    lines_scan_forward_wrapped(buffer, query, row, col).map(|found| (found, true))
}

fn lines_find_prev(
    buffer: &Buffer,
    query: &Query,
    row: usize,
    col: usize,
) -> Option<((usize, usize), bool)> {
    if let Some(found) = lines_scan_backward(buffer, query, row, col) {
        return Some((found, false));
    }
    lines_scan_backward_wrapped(buffer, query, row).map(|found| (found, true))
}

fn lines_first_from(
    buffer: &Buffer,
    query: &Query,
    row: usize,
    col: usize,
) -> Option<(usize, usize)> {
    buffer
        .line_slices()
        .enumerate()
        .skip(row)
        .find_map(|(r, slice)| {
            let bound = bound_forward_col(r, row, col);
            let text = slice_text(&slice);
            query.first_at_or_after(&text, bound).map(|c| (r, c))
        })
}

fn lines_scan_forward(
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
            query.first_at_or_after(&text, bound).map(|c| (r, c))
        })
}

fn bound_forward_col(line_row: usize, origin_row: usize, origin_col: usize) -> usize {
    if line_row == origin_row {
        return origin_col;
    }
    0
}

fn lines_scan_forward_wrapped(
    buffer: &Buffer,
    query: &Query,
    row: usize,
    col: usize,
) -> Option<(usize, usize)> {
    if let Some(found) = lines_scan_forward(buffer, query, 0, 0, row) {
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

fn lines_scan_backward(
    buffer: &Buffer,
    query: &Query,
    row: usize,
    col: usize,
) -> Option<(usize, usize)> {
    let mut found = None;
    for (r, slice) in buffer.line_slices().enumerate().take(row.saturating_add(1)) {
        let text = slice_text(&slice);
        let hit = lines_scan_backward_row(&text, query, r, row, col);
        if hit.is_some() {
            found = hit;
        }
    }
    found
}

fn lines_scan_backward_row(
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

fn lines_scan_backward_wrapped(
    buffer: &Buffer,
    query: &Query,
    row: usize,
) -> Option<(usize, usize)> {
    let mut found = None;
    for (r, slice) in buffer.line_slices().enumerate().skip(row.saturating_add(1)) {
        let text = slice_text(&slice);
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

fn lines_count_matches(buffer: &Buffer, query: &Query) -> usize {
    buffer
        .line_slices()
        .map(|slice| query.count_in_line(&slice_text(&slice)))
        .sum()
}

fn lines_collect_matches(
    buffer: &Buffer,
    query: &Query,
    start_row: usize,
    end_row: usize,
) -> Vec<MatchSpan> {
    buffer
        .line_slices()
        .enumerate()
        .skip(start_row)
        .take(end_row.saturating_sub(start_row))
        .flat_map(|(row, slice)| {
            let text = slice_text(&slice);
            let width = text.chars().count();
            query
                .spans_in_line(&text)
                .into_iter()
                .map(move |(start, end)| MatchSpan {
                    row,
                    start,
                    end: end.min(width),
                    current: false,
                })
        })
        .collect()
}

fn lines_matches_before(buffer: &Buffer, query: &Query, row: usize, col: usize) -> usize {
    let mut total: usize = 0;
    for (r, slice) in buffer.line_slices().enumerate().take(row.saturating_add(1)) {
        let text = slice_text(&slice);
        total = total.saturating_add(lines_row_before(&text, query, r, row, col));
    }
    total
}

fn lines_row_before(
    text: &str,
    query: &Query,
    line_row: usize,
    origin_row: usize,
    origin_col: usize,
) -> usize {
    if line_row == origin_row {
        return query.count_before(text, origin_col);
    }
    query.count_in_line(text)
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

    #[test]
    fn counts_all_matches() {
        let lines = lines_of("ar ar\nar");
        assert_eq!(count_matches(&lines, "ar"), 3);
        assert_eq!(count_matches(&lines, ""), 0);
        assert_eq!(count_matches(&lines, "zzz"), 0);
    }

    #[test]
    fn ranks_match_at_cursor() {
        let lines = lines_of("ar ar\nar");
        assert_eq!(match_rank(&lines, "ar", 0, 0, 3), 1);
        assert_eq!(match_rank(&lines, "ar", 0, 3, 3), 2);
        assert_eq!(match_rank(&lines, "ar", 1, 0, 3), 3);
    }

    #[test]
    fn rank_clamps_to_total() {
        let lines = lines_of("ar ar");
        assert_eq!(match_rank(&lines, "ar", 0, 5, 2), 2);
        assert_eq!(match_rank(&lines, "", 0, 0, 0), 0);
    }

    #[test]
    fn counts_unicode_pattern() {
        let lines = lines_of("CAFÉ café");
        assert_eq!(count_matches(&lines, "café"), 2);
        assert_eq!(match_rank(&lines, "café", 0, 5, 2), 2);
    }

    #[test]
    fn collects_viewport_spans() {
        let lines = lines_of("ar ar\nar");
        assert_eq!(
            collect_matches(&lines, "ar", 0, 2),
            vec![
                MatchSpan {
                    row: 0,
                    start: 0,
                    end: 2,
                    current: false,
                },
                MatchSpan {
                    row: 0,
                    start: 3,
                    end: 5,
                    current: false,
                },
                MatchSpan {
                    row: 1,
                    start: 0,
                    end: 2,
                    current: false,
                },
            ]
        );
        assert!(collect_matches(&lines, "ar", 1, 1).is_empty());
        assert!(collect_matches(&lines, "", 0, 2).is_empty());
    }

    #[test]
    fn collects_unicode_spans() {
        let lines = lines_of("CAFÉ x");
        assert_eq!(
            collect_matches(&lines, "café", 0, 1),
            vec![MatchSpan {
                row: 0,
                start: 0,
                end: 4,
                current: false,
            }]
        );
    }
}
