use ropey::RopeSlice;

pub struct Query {
    text: String,
    sensitive: bool,
    ascii_fold: bool,
}

impl Query {
    pub fn new(pattern: &str) -> Self {
        if pattern_has_upper(pattern) {
            Self {
                text: pattern.to_string(),
                sensitive: true,
                ascii_fold: false,
            }
        } else {
            Self {
                text: pattern.to_lowercase(),
                sensitive: false,
                ascii_fold: pattern.is_ascii(),
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn needs_folded_lines(&self) -> bool {
        !self.sensitive && !self.ascii_fold
    }

    pub fn first_at_or_after(&self, line: &str, bound_col: usize) -> Option<usize> {
        if self.needs_folded_lines() {
            return folded_first_at_or_after(&self.text, line, bound_col);
        }
        byte_first_at_or_after(self.needle(), self.fold_bytes(), line, bound_col)
    }

    pub fn first_before(&self, line: &str, bound_col: usize) -> Option<usize> {
        if self.needs_folded_lines() {
            return folded_first_before(&self.text, line, bound_col);
        }
        let bound = byte_of_col(line, bound_col);
        find_first_in(line.as_bytes(), self.needle(), self.fold_bytes(), 0, bound)
            .map(|byte| col_of_byte(line, byte))
    }

    pub fn last_before(&self, line: &str, bound_col: usize) -> Option<usize> {
        if self.needs_folded_lines() {
            return folded_last_before(&self.text, line, bound_col);
        }
        let bound = byte_of_col(line, bound_col);
        find_last_in(line.as_bytes(), self.needle(), self.fold_bytes(), 0, bound)
            .map(|byte| col_of_byte(line, byte))
    }

    pub fn last_in_line(&self, line: &str) -> Option<usize> {
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

    pub fn count_in_line(&self, line: &str) -> usize {
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

    pub fn count_before(&self, line: &str, bound_col: usize) -> usize {
        if self.needs_folded_lines() {
            return folded_count_before(&self.text, line, bound_col);
        }
        let bound = byte_of_col(line, bound_col);
        count_starts(line.as_bytes(), self.needle(), self.fold_bytes(), 0, bound)
    }

    fn needle(&self) -> &[u8] {
        self.text.as_bytes()
    }

    fn fold_bytes(&self) -> bool {
        !self.sensitive
    }
}

pub fn first_in_range(
    slice: &RopeSlice,
    query: &Query,
    from_byte: usize,
    end_byte: usize,
) -> Option<usize> {
    if query.is_empty() {
        return None;
    }
    drive_first(
        window_chunks(slice, from_byte, end_byte),
        query.text.as_bytes(),
        !query.sensitive,
    )
    .map(|hit| from_byte.min(slice.len_bytes()).saturating_add(hit))
}

pub fn count_in_range(
    slice: &RopeSlice,
    query: &Query,
    from_byte: usize,
    end_byte: usize,
) -> usize {
    if query.is_empty() {
        return 0;
    }
    drive_count(
        window_chunks(slice, from_byte, end_byte),
        query.text.as_bytes(),
        !query.sensitive,
    )
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
    drive_last(
        window_chunks(slice, from_byte, end_byte),
        query.text.as_bytes(),
        !query.sensitive,
    )
    .map(|hit| from_byte.min(slice.len_bytes()).saturating_add(hit))
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

fn drive_first<'a, I>(chunks: I, needle: &[u8], fold: bool) -> Option<usize>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    if needle.is_empty() {
        return None;
    }
    scan_views(chunks, needle.len(), |base, view, carry_len| {
        let mut from = 0;
        while let Some(hit) = find_first_in(view, needle, fold, from, view.len()) {
            if starts_in_scanned_prefix(hit, carry_len, needle.len()) {
                from = hit.saturating_add(1);
                continue;
            }
            return Some(base.saturating_add(hit));
        }
        None
    })
}

fn drive_last<'a, I>(chunks: I, needle: &[u8], fold: bool) -> Option<usize>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    if needle.is_empty() {
        return None;
    }
    let mut found = None;
    let _ = scan_views(chunks, needle.len(), |base, view, carry_len| {
        let mut from = 0;
        while let Some(hit) = find_first_in(view, needle, fold, from, view.len()) {
            if !starts_in_scanned_prefix(hit, carry_len, needle.len()) {
                found = found.max(Some(base.saturating_add(hit)));
            }
            from = hit.saturating_add(1);
        }
        None
    });
    found
}

fn drive_count<'a, I>(chunks: I, needle: &[u8], fold: bool) -> usize
where
    I: Iterator<Item = (usize, &'a str)>,
{
    if needle.is_empty() {
        return 0;
    }
    let mut total: usize = 0;
    let _ = scan_views(chunks, needle.len(), |_, view, carry_len| {
        let mut from = 0;
        while let Some(hit) = find_first_in(view, needle, fold, from, view.len()) {
            if !starts_in_scanned_prefix(hit, carry_len, needle.len()) {
                total = total.saturating_add(1);
            }
            from = hit.saturating_add(1);
        }
        None
    });
    total
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

    #[test]
    fn drive_spans_chunk_boundary() {
        let chunks = [(0, "ab"), (2, "cd")];
        assert_eq!(drive_first(chunks.into_iter(), b"bc", false), Some(1));
        assert_eq!(drive_last(chunks.into_iter(), b"bc", false), Some(1));
    }

    #[test]
    fn drive_keeps_first_and_last() {
        let chunks = [(0, "ar "), (3, "ar ar")];
        assert_eq!(drive_first(chunks.into_iter(), b"ar", false), Some(0));
        assert_eq!(drive_last(chunks.into_iter(), b"ar", false), Some(6));
    }

    #[test]
    fn drive_skips_verified_prefix() {
        let chunks = [(0, "aaa"), (3, "aaa")];
        assert_eq!(drive_first(chunks.into_iter(), b"aa", false), Some(0));
        assert_eq!(drive_last(chunks.into_iter(), b"aa", false), Some(4));
    }

    #[test]
    fn range_searches_rope_slice() {
        let buffer = buffer_of("foo\nbar");
        let slice = buffer.rope_slice();
        let query = Query::new("ba");
        assert_eq!(
            first_in_range(&slice, &query, 0, slice.len_bytes()),
            Some(4)
        );
        let query = Query::new("BA");
        assert_eq!(first_in_range(&slice, &query, 0, slice.len_bytes()), None);
        let query = Query::new("AR");
        assert_eq!(last_in_range(&slice, &query, 0, slice.len_bytes()), None);
    }

    #[test]
    fn range_folds_case() {
        let buffer = buffer_of("FOO bar");
        let slice = buffer.rope_slice();
        let query = Query::new("foo");
        assert_eq!(
            first_in_range(&slice, &query, 0, slice.len_bytes()),
            Some(0)
        );
        let query = Query::new("BAR");
        assert_eq!(first_in_range(&slice, &query, 0, slice.len_bytes()), None);
    }

    #[test]
    fn unicode_pattern_uses_folded_lines() {
        let query = Query::new("café");
        assert!(query.needs_folded_lines());
        assert_eq!(query.first_at_or_after("CAFÉ x", 0), Some(0));
        let ascii = Query::new("cafe");
        assert!(!ascii.needs_folded_lines());
    }

    #[test]
    fn range_counts_all_matches() {
        let buffer = buffer_of("ar ar\nar");
        let slice = buffer.rope_slice();
        let query = Query::new("ar");
        assert_eq!(count_in_range(&slice, &query, 0, slice.len_bytes()), 3);
        assert_eq!(count_in_range(&slice, &query, 0, 0), 0);
        let empty = Query::new("");
        assert_eq!(count_in_range(&slice, &empty, 0, slice.len_bytes()), 0);
    }

    #[test]
    fn range_counts_across_chunk_edges() {
        let chunks = [(0, "aaa"), (3, "aaa")];
        assert_eq!(drive_count(chunks.into_iter(), b"aa", false), 5);
    }

    #[test]
    fn line_counts_matches() {
        let query = Query::new("ar");
        assert_eq!(query.count_in_line("ar ar"), 2);
        assert_eq!(query.count_in_line("foo"), 0);
        assert_eq!(query.count_before("ar ar", 4), 2);
        assert_eq!(query.count_before("ar ar", 0), 0);
    }

    #[test]
    fn line_primitives_match_columns() {
        let query = Query::new("ar");
        assert_eq!(query.first_at_or_after("ar ar", 1), Some(3));
        assert_eq!(query.first_before("ar ar", 4), Some(0));
        assert_eq!(query.last_before("ar ar", 4), Some(3));
        assert_eq!(query.last_in_line("ar ar"), Some(3));
    }
}
