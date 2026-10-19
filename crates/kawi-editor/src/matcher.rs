use ropey::RopeSlice;

use crate::words::is_word_char;

pub struct Query {
    text: String,
    sensitive: bool,
    ascii_fold: bool,
    whole_word: bool,
}

impl Query {
    pub fn new(pattern: &str, sensitive: bool, whole_word: bool) -> Self {
        if sensitive {
            Self {
                text: pattern.to_string(),
                sensitive: true,
                ascii_fold: false,
                whole_word,
            }
        } else {
            Self {
                text: pattern.to_lowercase(),
                sensitive: false,
                ascii_fold: pattern.is_ascii(),
                whole_word,
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn needs_folded_scan(&self) -> bool {
        self.needs_folded_lines()
    }

    fn needs_folded_lines(&self) -> bool {
        !self.sensitive && !self.ascii_fold
    }

    fn raw_first_at_or_after(&self, line: &str, bound_col: usize) -> Option<usize> {
        if self.needs_folded_lines() {
            return folded_first_at_or_after(&self.text, line, bound_col);
        }
        byte_first_at_or_after(self.needle(), self.fold_bytes(), line, bound_col)
    }

    fn raw_first_before(&self, line: &str, bound_col: usize) -> Option<usize> {
        if self.needs_folded_lines() {
            return folded_first_before(&self.text, line, bound_col);
        }
        let bound = byte_of_col(line, bound_col);
        find_first_in(line.as_bytes(), self.needle(), self.fold_bytes(), 0, bound)
            .map(|byte| col_of_byte(line, byte))
    }

    fn raw_last_before(&self, line: &str, bound_col: usize) -> Option<usize> {
        if self.needs_folded_lines() {
            return folded_last_before(&self.text, line, bound_col);
        }
        let bound = byte_of_col(line, bound_col);
        find_last_in(line.as_bytes(), self.needle(), self.fold_bytes(), 0, bound)
            .map(|byte| col_of_byte(line, byte))
    }

    fn raw_last_in_line(&self, line: &str) -> Option<usize> {
        if self.needs_folded_lines() {
            return folded_last_in_line(&self.text, line);
        }
        find_last_in(
            line.as_bytes(),
            self.needle(),
            self.fold_bytes(),
            0,
            line.len(),
        )
        .map(|byte| col_of_byte(line, byte))
    }

    fn raw_count_in_line(&self, line: &str) -> usize {
        if self.needs_folded_lines() {
            return folded_count_in_line(&self.text, line);
        }
        count_starts(
            line.as_bytes(),
            self.needle(),
            self.fold_bytes(),
            0,
            line.len(),
        )
    }

    fn raw_count_before(&self, line: &str, bound_col: usize) -> usize {
        if self.needs_folded_lines() {
            return folded_count_before(&self.text, line, bound_col);
        }
        let bound = byte_of_col(line, bound_col);
        count_starts(line.as_bytes(), self.needle(), self.fold_bytes(), 0, bound)
    }

    fn raw_spans_in_line(&self, line: &str) -> Vec<(usize, usize)> {
        if self.needs_folded_lines() {
            return folded_spans_in_line(&self.text, line);
        }
        let mut spans = Vec::new();
        let mut from = 0;
        while let Some(hit) = find_first_in(
            line.as_bytes(),
            self.needle(),
            self.fold_bytes(),
            from,
            line.len(),
        ) {
            spans.push((
                col_of_byte(line, hit),
                col_of_byte(line, hit.saturating_add(self.needle().len())),
            ));
            from = hit.saturating_add(1);
        }
        spans
    }

    pub fn first_at_or_after(&self, line: &str, bound_col: usize) -> Option<usize> {
        if !self.whole_word {
            return self.raw_first_at_or_after(line, bound_col);
        }
        let mut from = bound_col;
        loop {
            let col = self.raw_first_at_or_after(line, from)?;
            if self.accepts(line, col) {
                return Some(col);
            }
            from = col.saturating_add(1);
        }
    }

    pub fn first_before(&self, line: &str, bound_col: usize) -> Option<usize> {
        if !self.whole_word {
            return self.raw_first_before(line, bound_col);
        }
        let mut from = bound_col;
        loop {
            let col = self.raw_first_before(line, from)?;
            if self.accepts(line, col) {
                return Some(col);
            }
            from = col;
        }
    }

    pub fn last_before(&self, line: &str, bound_col: usize) -> Option<usize> {
        if !self.whole_word {
            return self.raw_last_before(line, bound_col);
        }
        let mut from = bound_col;
        loop {
            let col = self.raw_last_before(line, from)?;
            if self.accepts(line, col) {
                return Some(col);
            }
            from = col;
        }
    }

    pub fn last_in_line(&self, line: &str) -> Option<usize> {
        if !self.whole_word {
            return self.raw_last_in_line(line);
        }
        self.spans_in_line(line).last().map(|span| span.0)
    }

    pub fn count_in_line(&self, line: &str) -> usize {
        if !self.whole_word {
            return self.raw_count_in_line(line);
        }
        self.spans_in_line(line).len()
    }

    pub fn count_before(&self, line: &str, bound_col: usize) -> usize {
        if !self.whole_word {
            return self.raw_count_before(line, bound_col);
        }
        self.spans_in_line(line)
            .into_iter()
            .filter(|(start, _)| *start < bound_col)
            .count()
    }

    pub fn spans_in_line(&self, line: &str) -> Vec<(usize, usize)> {
        let spans = self.raw_spans_in_line(line);
        if !self.whole_word {
            return spans;
        }
        spans
            .into_iter()
            .filter(|(start, _)| self.accepts(line, *start))
            .collect()
    }

    fn accepts(&self, line: &str, col: usize) -> bool {
        if !self.whole_word {
            return true;
        }
        let before = line.chars().nth(col.wrapping_sub(1));
        let end = col.saturating_add(self.text.chars().count());
        let after = line.chars().nth(end);
        boundaries_ok(before, after)
    }

    fn needle(&self) -> &[u8] {
        self.text.as_bytes()
    }

    fn fold_bytes(&self) -> bool {
        !self.sensitive
    }
}

pub fn scan_in_range(
    slice: &RopeSlice,
    query: &Query,
    from_byte: usize,
    end_byte: usize,
    mut visit: impl FnMut(usize) -> bool,
) -> bool {
    if query.is_empty() {
        return true;
    }
    let total = slice.len_bytes();
    let from = from_byte.min(total);
    let limit = end_byte.min(total);
    let needle = query.text.as_bytes();
    let window_end = limit
        .saturating_add(needle.len().saturating_sub(1))
        .min(total);
    let whole_word = query.whole_word;
    scan_chunks(
        window_chunks(slice, from, window_end),
        needle,
        !query.sensitive,
        |hit| {
            let absolute = from.saturating_add(hit);
            if absolute >= limit {
                return false;
            }
            if whole_word && !word_bounded(slice, absolute, needle.len()) {
                return true;
            }
            visit(absolute)
        },
    )
}

pub fn first_in_range(
    slice: &RopeSlice,
    query: &Query,
    from_byte: usize,
    end_byte: usize,
) -> Option<usize> {
    let mut found = None;
    scan_in_range(slice, query, from_byte, end_byte, |hit| {
        found = Some(hit);
        false
    });
    found
}

pub fn spans_in_range(
    slice: &RopeSlice,
    query: &Query,
    from_byte: usize,
    end_byte: usize,
) -> Vec<(usize, usize)> {
    let needle_len = query.text.len();
    let mut spans = Vec::new();
    scan_in_range(slice, query, from_byte, end_byte, |hit| {
        spans.push((hit, hit.saturating_add(needle_len)));
        true
    });
    spans
}

pub fn count_in_range(
    slice: &RopeSlice,
    query: &Query,
    from_byte: usize,
    end_byte: usize,
) -> usize {
    let mut total = 0usize;
    scan_in_range(slice, query, from_byte, end_byte, |_| {
        total = total.saturating_add(1);
        true
    });
    total
}

pub fn last_in_range(
    slice: &RopeSlice,
    query: &Query,
    from_byte: usize,
    end_byte: usize,
) -> Option<usize> {
    if query.is_empty() {
        return None;
    }
    let total = slice.len_bytes();
    let from = from_byte.min(total);
    let limit = end_byte.min(total);
    let needle = query.text.as_bytes();
    let overlap = needle.len().saturating_sub(1);
    let window_end = limit.saturating_add(overlap).min(total);
    let chunks: Vec<(usize, &str)> = window_chunks(slice, from, window_end).collect();
    let mut follow: Vec<u8> = Vec::new();
    for (offset, text) in chunks.iter().rev() {
        let mut view: Vec<u8> = Vec::with_capacity(text.len().saturating_add(overlap));
        view.extend_from_slice(text.as_bytes());
        view.extend_from_slice(&follow);
        let base = from.saturating_add(*offset);
        if let Some(absolute) = last_start_in_chunk(slice, query, &view, text.len(), base, limit) {
            return Some(absolute);
        }
        follow.clear();
        let keep = overlap.min(text.len());
        follow.extend_from_slice(&text.as_bytes()[..keep]);
    }
    None
}

fn last_start_in_chunk(
    slice: &RopeSlice,
    query: &Query,
    view: &[u8],
    chunk_len: usize,
    base: usize,
    limit: usize,
) -> Option<usize> {
    let needle = query.text.as_bytes();
    let whole_word = query.whole_word;
    let mut found = None;
    let mut from = 0;
    while let Some(hit) = find_first_in(view, needle, !query.sensitive, from, chunk_len) {
        let absolute = base.saturating_add(hit);
        if absolute < limit && (!whole_word || word_bounded(slice, absolute, needle.len())) {
            found = Some(absolute);
        }
        from = hit.saturating_add(1);
    }
    found
}

fn window_chunks<'a>(
    slice: &'a RopeSlice<'a>,
    from_byte: usize,
    end_byte: usize,
) -> impl Iterator<Item = (usize, &'a str)> {
    let total = slice.len_bytes();
    let from = from_byte.min(total);
    let end = end_byte.min(total).max(from);
    let window = slice.byte_slice(from..end);
    window.chunks().scan(0usize, |offset, chunk| {
        let start = *offset;
        *offset = offset.saturating_add(chunk.len());
        Some((start, chunk))
    })
}

fn scan_chunks<'a, I>(
    chunks: I,
    needle: &[u8],
    fold: bool,
    mut visit: impl FnMut(usize) -> bool,
) -> bool
where
    I: Iterator<Item = (usize, &'a str)>,
{
    if needle.is_empty() {
        return true;
    }
    let mut stopped = false;
    let _ = scan_views(chunks, needle.len(), |base, view, carry_len| {
        let mut from = 0;
        while let Some(hit) = find_first_in(view, needle, fold, from, view.len()) {
            if !starts_in_scanned_prefix(hit, carry_len, needle.len())
                && !visit(base.saturating_add(hit))
            {
                stopped = true;
                return Some(0);
            }
            from = hit.saturating_add(1);
        }
        None
    });
    !stopped
}

fn word_bounded(slice: &RopeSlice, byte: usize, len: usize) -> bool {
    let total = slice.len_bytes();
    let start = slice.byte_to_char(byte.min(total));
    let end = slice.byte_to_char(byte.saturating_add(len).min(total));
    let before = if start > 0 {
        slice.get_char(start - 1)
    } else {
        None
    };
    let after = slice.get_char(end);
    boundaries_ok(before, after)
}

fn scan_views<'a, I, F>(chunks: I, needle_len: usize, mut visit: F) -> Option<usize>
where
    I: Iterator<Item = (usize, &'a str)>,
    F: FnMut(usize, &[u8], usize) -> Option<usize>,
{
    let overlap = needle_len.saturating_sub(1);
    let mut carry: Vec<u8> = Vec::new();
    let mut view: Vec<u8> = Vec::new();
    for (offset, text) in chunks {
        let base = offset.saturating_sub(carry.len());
        view.clear();
        view.extend_from_slice(&carry);
        view.extend_from_slice(text.as_bytes());
        if let Some(found) = visit(base, &view, carry.len()) {
            return Some(found);
        }
        refresh_carry(&mut carry, &view, overlap);
    }
    None
}

fn starts_in_scanned_prefix(hit: usize, carry_len: usize, needle_len: usize) -> bool {
    hit < carry_len && hit.saturating_add(needle_len) <= carry_len
}

fn refresh_carry(carry: &mut Vec<u8>, view: &[u8], overlap: usize) {
    carry.clear();
    let keep = overlap.min(view.len());
    carry.extend_from_slice(&view[view.len().saturating_sub(keep)..]);
}

fn boundaries_ok(before: Option<char>, after: Option<char>) -> bool {
    !before.is_some_and(is_word_char) && !after.is_some_and(is_word_char)
}

fn matches_at(window: &[u8], needle: &[u8], fold: bool) -> bool {
    if window.len() < needle.len() {
        return false;
    }
    if fold {
        return folded_prefix_matches(window, needle);
    }
    window[..needle.len()] == needle[..]
}

fn folded_prefix_matches(window: &[u8], needle: &[u8]) -> bool {
    let mut index = 0;
    while index < needle.len() {
        if window[index].to_ascii_lowercase() != needle[index] {
            return false;
        }
        index = index.saturating_add(1);
    }
    true
}

fn find_first_in(
    haystack: &[u8],
    needle: &[u8],
    fold: bool,
    from: usize,
    end: usize,
) -> Option<usize> {
    let end = end.min(haystack.len());
    if needle.is_empty() || needle.len() > haystack.len() || from >= end {
        return None;
    }
    let last = end
        .saturating_sub(1)
        .min(haystack.len().saturating_sub(needle.len()));
    if from > last {
        return None;
    }
    let mut base = from;
    loop {
        let candidate = scan_head(&haystack[base..=last], needle[0], fold)?;
        let found = base.saturating_add(candidate);
        if matches_at(&haystack[found..], needle, fold) {
            return Some(found);
        }
        base = found.saturating_add(1);
        if base > last {
            return None;
        }
    }
}

fn find_last_in(
    haystack: &[u8],
    needle: &[u8],
    fold: bool,
    from: usize,
    end: usize,
) -> Option<usize> {
    let mut found = None;
    let mut next = from;
    while let Some(hit) = find_first_in(haystack, needle, fold, next, end) {
        found = Some(hit);
        next = hit.saturating_add(1);
    }
    found
}

fn count_starts(haystack: &[u8], needle: &[u8], fold: bool, from: usize, end: usize) -> usize {
    let mut total: usize = 0;
    let mut next = from;
    while let Some(hit) = find_first_in(haystack, needle, fold, next, end) {
        total = total.saturating_add(1);
        next = hit.saturating_add(1);
    }
    total
}

fn scan_head(window: &[u8], first: u8, fold: bool) -> Option<usize> {
    if fold {
        let upper = first.to_ascii_uppercase();
        if upper == first {
            return memchr::memchr(first, window);
        }
        return memchr::memchr2(first, upper, window);
    }
    memchr::memchr(first, window)
}

fn byte_first_at_or_after(
    needle: &[u8],
    fold: bool,
    line: &str,
    bound_col: usize,
) -> Option<usize> {
    let bound = byte_of_col(line, bound_col);
    find_first_in(line.as_bytes(), needle, fold, bound, line.len())
        .map(|byte| col_of_byte(line, byte))
}

fn folded_first_at_or_after(pattern: &str, line: &str, bound_col: usize) -> Option<usize> {
    let haystack = fold_case(line);
    let bound = byte_of_col(&haystack, bound_col);
    haystack
        .match_indices(pattern)
        .map(|(byte, _)| byte)
        .find(|byte| *byte >= bound)
        .map(|byte| col_of_byte(&haystack, byte))
}

fn folded_first_before(pattern: &str, line: &str, bound_col: usize) -> Option<usize> {
    let bound = byte_of_col(line, bound_col);
    let haystack = fold_case(line);
    haystack
        .match_indices(pattern)
        .map(|(byte, _)| byte)
        .take_while(|byte| *byte < bound)
        .next()
        .map(|byte| col_of_byte(&haystack, byte))
}

fn folded_last_before(pattern: &str, line: &str, bound_col: usize) -> Option<usize> {
    let haystack = fold_case(line);
    let bound = byte_of_col(&haystack, bound_col);
    haystack
        .match_indices(pattern)
        .map(|(byte, _)| byte)
        .take_while(|byte| *byte < bound)
        .last()
        .map(|byte| col_of_byte(&haystack, byte))
}

fn folded_last_in_line(pattern: &str, line: &str) -> Option<usize> {
    let haystack = fold_case(line);
    haystack
        .match_indices(pattern)
        .map(|(byte, _)| byte)
        .last()
        .map(|byte| col_of_byte(&haystack, byte))
}

fn folded_count_in_line(pattern: &str, line: &str) -> usize {
    fold_case(line).match_indices(pattern).count()
}

fn folded_count_before(pattern: &str, line: &str, bound_col: usize) -> usize {
    let haystack = fold_case(line);
    let bound = byte_of_col(&haystack, bound_col);
    haystack
        .match_indices(pattern)
        .map(|(byte, _)| byte)
        .take_while(|byte| *byte < bound)
        .count()
}

fn folded_spans_in_line(pattern: &str, line: &str) -> Vec<(usize, usize)> {
    let haystack = fold_case(line);
    haystack
        .match_indices(pattern)
        .map(|(byte, matched)| {
            (
                col_of_byte(&haystack, byte),
                col_of_byte(&haystack, byte.saturating_add(matched.len())),
            )
        })
        .collect()
}

fn byte_of_col(line: &str, col: usize) -> usize {
    line.char_indices()
        .nth(col)
        .map_or(line.len(), |(byte, _)| byte)
}

fn col_of_byte(line: &str, byte: usize) -> usize {
    line[..byte.min(line.len())].chars().count()
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

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_buffer::Buffer;

    fn buffer_of(text: &str) -> Buffer {
        Buffer::from_string(text)
    }

    #[test]
    fn core_finds_exact_bytes() {
        assert_eq!(find_first_in(b"foo bar", b"bar", false, 0, 7), Some(4));
        assert_eq!(find_first_in(b"foo bar", b"zzz", false, 0, 7), None);
    }

    #[test]
    fn core_folds_ascii_without_alloc() {
        assert_eq!(find_first_in(b"FOO bar", b"foo", true, 0, 7), Some(0));
        assert_eq!(find_first_in(b"FOO bar", b"bar", true, 0, 7), Some(4));
    }

    #[test]
    fn core_respects_range() {
        assert_eq!(find_first_in(b"ar ar", b"ar", false, 3, 5), Some(3));
        assert_eq!(find_first_in(b"ar ar", b"ar", false, 4, 5), None);
        assert_eq!(find_last_in(b"ar ar", b"ar", false, 0, 5), Some(3));
        assert_eq!(find_last_in(b"ar ar", b"ar", false, 0, 3), Some(0));
    }

    #[test]
    fn core_matches_utf8_bytes_exactly() {
        let line = "café x";
        assert_eq!(
            find_first_in(line.as_bytes(), "café".as_bytes(), false, 0, line.len()),
            Some(0)
        );
    }

    fn scan_hits(chunks: &[(usize, &str)], needle: &[u8]) -> Vec<usize> {
        let mut hits = Vec::new();
        scan_chunks(chunks.iter().copied(), needle, false, |hit| {
            hits.push(hit);
            true
        });
        hits
    }

    #[test]
    fn scan_finds_matches_across_chunk_boundary() {
        assert_eq!(scan_hits(&[(0, "ab"), (2, "cd")], b"bc"), vec![1]);
    }

    #[test]
    fn scan_keeps_all_hits_in_order() {
        assert_eq!(scan_hits(&[(0, "ar "), (3, "ar ar")], b"ar"), vec![0, 3, 6]);
    }

    #[test]
    fn scan_finds_overlapping_matches_across_chunks() {
        assert_eq!(
            scan_hits(&[(0, "aaa"), (3, "aaa")], b"aa"),
            vec![0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn scan_stops_when_visitor_returns_false() {
        let mut seen = Vec::new();
        let completed = scan_chunks([(0, "aaa")].iter().copied(), b"a", false, |hit| {
            seen.push(hit);
            false
        });
        assert!(!completed);
        assert_eq!(seen, vec![0]);
    }

    #[test]
    fn range_searches_rope_slice() {
        let buffer = buffer_of("foo\nbar");
        let slice = buffer.rope_slice();
        let query = Query::new("ba", false, false);
        assert_eq!(
            first_in_range(&slice, &query, 0, slice.len_bytes()),
            Some(4)
        );
        let query = Query::new("BA", true, false);
        assert_eq!(first_in_range(&slice, &query, 0, slice.len_bytes()), None);
        let query = Query::new("AR", true, false);
        assert_eq!(last_in_range(&slice, &query, 0, slice.len_bytes()), None);
    }

    #[test]
    fn range_folds_case() {
        let buffer = buffer_of("FOO bar");
        let slice = buffer.rope_slice();
        let query = Query::new("foo", false, false);
        assert_eq!(
            first_in_range(&slice, &query, 0, slice.len_bytes()),
            Some(0)
        );
        let query = Query::new("BAR", true, false);
        assert_eq!(first_in_range(&slice, &query, 0, slice.len_bytes()), None);
    }

    #[test]
    fn unicode_pattern_uses_folded_lines() {
        let query = Query::new("café", false, false);
        assert!(query.needs_folded_scan());
        assert_eq!(query.first_at_or_after("CAFÉ x", 0), Some(0));
        let ascii = Query::new("cafe", false, false);
        assert!(!ascii.needs_folded_scan());
    }

    #[test]
    fn range_counts_all_matches() {
        let buffer = buffer_of("ar ar\nar");
        let slice = buffer.rope_slice();
        let query = Query::new("ar", false, false);
        assert_eq!(count_in_range(&slice, &query, 0, slice.len_bytes()), 3);
        assert_eq!(count_in_range(&slice, &query, 0, 0), 0);
        let empty = Query::new("", false, false);
        assert_eq!(count_in_range(&slice, &empty, 0, slice.len_bytes()), 0);
    }

    #[test]
    fn range_counts_across_chunk_edges() {
        assert_eq!(
            scan_hits(&[(0, "aaa"), (3, "aaa")], b"aa"),
            vec![0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn range_collects_spans() {
        let buffer = buffer_of("ar ar\nar");
        let slice = buffer.rope_slice();
        let query = Query::new("ar", false, false);
        assert_eq!(
            spans_in_range(&slice, &query, 0, slice.len_bytes()),
            vec![(0, 2), (3, 5), (6, 8)]
        );
        assert!(spans_in_range(&slice, &query, 0, 0).is_empty());
    }

    #[test]
    fn spans_cover_chunk_edges_once() {
        let hits = scan_hits(&[(0, "aaa"), (3, "aaa")], b"aa");
        let spans: Vec<(usize, usize)> = hits.iter().map(|hit| (*hit, hit + 2)).collect();
        assert_eq!(spans, vec![(0, 2), (1, 3), (2, 4), (3, 5), (4, 6)]);
    }

    #[test]
    fn line_collects_spans() {
        let query = Query::new("ar", false, false);
        assert_eq!(query.spans_in_line("ar ar"), vec![(0, 2), (3, 5)]);
        assert!(query.spans_in_line("foo").is_empty());
    }

    #[test]
    fn line_counts_matches() {
        let query = Query::new("ar", false, false);
        assert_eq!(query.count_in_line("ar ar"), 2);
        assert_eq!(query.count_in_line("foo"), 0);
        assert_eq!(query.count_before("ar ar", 4), 2);
        assert_eq!(query.count_before("ar ar", 0), 0);
    }

    #[test]
    fn whole_word_matches_only_bounded_words() {
        let query = Query::new("foo", true, true);
        assert_eq!(
            query.spans_in_line("foo foobar foo"),
            vec![(0, 3), (11, 14)]
        );
        assert_eq!(query.count_in_line("foo foobar foo"), 2);
        assert_eq!(query.first_at_or_after("foobar foo", 0), Some(7));
        assert_eq!(query.last_before("foo foobar foo", 11), Some(0));
        assert_eq!(query.last_in_line("foo foobar foo"), Some(11));
        assert_eq!(query.count_before("foo foobar foo", 11), 1);
        assert_eq!(query.count_before("foo foobar foo", 12), 2);
    }

    #[test]
    fn whole_word_respects_underscores_and_digits() {
        let query = Query::new("foo", true, true);
        assert!(query.spans_in_line("foo_bar").is_empty());
        assert!(query.spans_in_line("foo1").is_empty());
        assert_eq!(query.spans_in_line("(foo)"), vec![(1, 4)]);
    }

    #[test]
    fn line_primitives_match_columns() {
        let query = Query::new("ar", false, false);
        assert_eq!(query.first_at_or_after("ar ar", 1), Some(3));
        assert_eq!(query.first_before("ar ar", 4), Some(0));
        assert_eq!(query.last_before("ar ar", 4), Some(3));
        assert_eq!(query.last_in_line("ar ar"), Some(3));
    }
}
