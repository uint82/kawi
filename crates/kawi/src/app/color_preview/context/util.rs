use std::ops::Range;

pub(super) fn merge_ranges(mut ranges: Vec<Range<usize>>) -> Vec<Range<usize>> {
    ranges.sort_by_key(|range| range.start);
    let mut merged: Vec<Range<usize>> = Vec::new();
    for range in ranges {
        if range.start >= range.end {
            continue;
        }
        if let Some(last) = merged.last_mut() {
            if range.start <= last.end {
                last.end = last.end.max(range.end);
                continue;
            }
        }
        merged.push(range);
    }
    merged
}

pub(super) fn find_bytes(bytes: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    find_window(bytes, needle, from, |window| window == needle)
}

pub(super) fn find_ignore_ascii_case(bytes: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    find_window(bytes, needle, from, |window| {
        window.eq_ignore_ascii_case(needle)
    })
}

fn find_window(
    bytes: &[u8],
    needle: &[u8],
    from: usize,
    matches: impl Fn(&[u8]) -> bool,
) -> Option<usize> {
    bytes
        .get(from..)?
        .windows(needle.len())
        .position(matches)
        .map(|offset| from.saturating_add(offset))
}

pub(super) fn find_byte(bytes: &[u8], from: usize, target: u8) -> Option<usize> {
    bytes
        .get(from..)?
        .iter()
        .position(|byte| *byte == target)
        .map(|offset| from.saturating_add(offset))
}

pub(super) fn skip_whitespace(bytes: &[u8], from: usize) -> usize {
    let mut index = from;
    while bytes
        .get(index)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        index = index.saturating_add(1);
    }
    index
}
