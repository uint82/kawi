use kawi_buffer::Buffer;

use crate::words::{class_of, remainder_blank, start_blank, WordClass};

pub mod brackets;
pub mod paragraph;
pub mod quotes;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AroundSpan {
    pub anchor: (usize, usize),
    pub start: (usize, usize),
    pub end: (usize, usize),
}

impl AroundSpan {
    pub fn spans_lines(&self, buffer: &Buffer) -> bool {
        self.end.0 > self.start.0
            && remainder_blank(buffer, self.end)
            && start_blank(buffer, self.start)
    }
}

pub fn iw_range(buffer: &Buffer, row: usize, col: usize) -> Option<AroundSpan> {
    let text = line_chars(buffer, row)?;
    let class = class_of(*text.get(col)?);
    let start = span_start(&text, col, class);
    let end = span_end(&text, col, class).saturating_add(1);
    Some(AroundSpan {
        anchor: (row, start),
        start: (row, start),
        end: (row, end),
    })
}

pub fn aw_range(buffer: &Buffer, row: usize, col: usize) -> Option<AroundSpan> {
    let line = buffer.line_cow(row)?;
    if line.is_empty() {
        return aw_scan(buffer, row, col);
    }
    let text: Vec<char> = line.chars().collect();
    let class = class_of(*text.get(col)?);
    if class == WordClass::Space {
        return aw_scan(buffer, row, col);
    }
    aw_from_word(&text, row, col, class)
}

fn aw_from_word(text: &[char], row: usize, col: usize, class: WordClass) -> Option<AroundSpan> {
    let start = span_start(text, col, class);
    let end = span_end(text, col, class);
    match trailing_spaces(text, end.saturating_add(1)) {
        Some(last) => Some(AroundSpan {
            anchor: (row, start),
            start: (row, start),
            end: (row, last.saturating_add(1)),
        }),
        None => {
            let head = leading_head(start, leading_start(text, start));
            Some(AroundSpan {
                anchor: (row, head),
                start: (row, head),
                end: (row, end.saturating_add(1)),
            })
        }
    }
}

fn leading_head(start: usize, head: usize) -> usize {
    if head == 0 && start > 0 {
        return start;
    }
    head
}

enum ScanFound {
    Word { row: usize, end: usize },
    Empty { row: usize },
}

fn aw_scan(buffer: &Buffer, row: usize, col: usize) -> Option<AroundSpan> {
    let anchor = (row, col);
    let eat = rewind_start(buffer, row, col);
    match scan_forward(buffer, row, col)? {
        ScanFound::Word { row: found, end } => Some(AroundSpan {
            anchor,
            start: eat,
            end: (found, end),
        }),
        ScanFound::Empty { row: empty } => Some(AroundSpan {
            anchor,
            start: eat,
            end: (empty, 0),
        }),
    }
}

fn scan_forward(buffer: &Buffer, row: usize, col: usize) -> Option<ScanFound> {
    let mut current = row;
    let mut from = col;
    loop {
        let text = line_chars(buffer, current)?;
        if current != row && text.is_empty() {
            return Some(ScanFound::Empty { row: current });
        }
        if let Some(start) = content_start(&text, from) {
            let end = span_end(&text, start, class_of(text[start])).saturating_add(1);
            return Some(ScanFound::Word { row: current, end });
        }
        current = current.saturating_add(1);
        from = 0;
    }
}

fn rewind_start(buffer: &Buffer, row: usize, col: usize) -> (usize, usize) {
    let Some(line) = buffer.line_cow(row) else {
        return (row, col);
    };
    let text: Vec<char> = line.chars().collect();
    let mut start = col.min(text.len());
    while start > 0 && text[start.saturating_sub(1)].is_whitespace() {
        start = start.saturating_sub(1);
    }
    (row, start)
}

fn leading_start(text: &[char], start: usize) -> usize {
    let mut head = start;
    while head > 0 && text[head.saturating_sub(1)].is_whitespace() {
        head = head.saturating_sub(1);
    }
    head
}

fn content_start(text: &[char], from: usize) -> Option<usize> {
    (from..text.len()).find(|idx| !text[*idx].is_whitespace())
}

fn trailing_spaces(text: &[char], from: usize) -> Option<usize> {
    if text.get(from).is_none_or(|ch| !ch.is_whitespace()) {
        return None;
    }
    let mut last = from;
    while last.saturating_add(1) < text.len() && text[last.saturating_add(1)].is_whitespace() {
        last = last.saturating_add(1);
    }
    Some(last)
}

fn line_chars(buffer: &Buffer, row: usize) -> Option<Vec<char>> {
    buffer.line_cow(row).map(|line| line.chars().collect())
}

fn span_start(text: &[char], col: usize, class: WordClass) -> usize {
    let mut start = col;
    while start > 0 && class_of(text[start.saturating_sub(1)]) == class {
        start = start.saturating_sub(1);
    }
    start
}

fn span_end(text: &[char], col: usize, class: WordClass) -> usize {
    let mut end = col;
    while end.saturating_add(1) < text.len() && class_of(text[end.saturating_add(1)]) == class {
        end = end.saturating_add(1);
    }
    end
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_buffer::Buffer;

    fn lines_of(text: &str) -> Buffer {
        Buffer::from_string(text)
    }

    fn around(anchor: (usize, usize), start: (usize, usize), end: (usize, usize)) -> AroundSpan {
        AroundSpan { anchor, start, end }
    }

    #[test]
    fn aw_at_end_takes_leading_space() {
        let lines = lines_of("foo bar");
        assert_eq!(aw_range(&lines, 0, 4), Some(around((0, 3), (0, 3), (0, 7))));
        let lines = lines_of("xx foo,bar");
        assert_eq!(aw_range(&lines, 0, 3), Some(around((0, 2), (0, 2), (0, 6))));
        let lines = lines_of("foo,bar");
        assert_eq!(aw_range(&lines, 0, 0), Some(around((0, 0), (0, 0), (0, 3))));
    }

    #[test]
    fn aw_crosses_lines_from_space() {
        let lines = lines_of("foo   \nbar");
        assert_eq!(aw_range(&lines, 0, 4), Some(around((0, 4), (0, 3), (1, 3))));
        let lines = lines_of("foo   \n\nbar");
        assert_eq!(aw_range(&lines, 0, 4), Some(around((0, 4), (0, 3), (1, 0))));
    }

    #[test]
    fn aw_keeps_indent_at_line_start() {
        let lines = lines_of("   foo\nbar");
        assert_eq!(aw_range(&lines, 0, 3), Some(around((0, 3), (0, 3), (0, 6))));
        let lines = lines_of("   foo");
        assert_eq!(aw_range(&lines, 0, 3), Some(around((0, 3), (0, 3), (0, 6))));
    }

    #[test]
    fn aw_on_empty_line_stops_at_empty() {
        let lines = lines_of("\n\nfoo");
        assert_eq!(aw_range(&lines, 0, 0), Some(around((0, 0), (0, 0), (1, 0))));
        let lines = lines_of("\n\n\nfoo");
        assert_eq!(aw_range(&lines, 0, 0), Some(around((0, 0), (0, 0), (1, 0))));
    }

    #[test]
    fn aw_on_empty_line_takes_next_word() {
        let lines = lines_of("\nfoo");
        assert_eq!(aw_range(&lines, 0, 0), Some(around((0, 0), (0, 0), (1, 3))));
        let lines = lines_of("\nfoo bar");
        assert_eq!(aw_range(&lines, 0, 0), Some(around((0, 0), (0, 0), (1, 3))));
        let lines = lines_of("foo\n\nbar");
        assert_eq!(aw_range(&lines, 1, 0), Some(around((1, 0), (1, 0), (2, 3))));
    }

    #[test]
    fn aw_on_space_takes_following_word() {
        let lines = lines_of("foo bar");
        assert_eq!(aw_range(&lines, 0, 3), Some(around((0, 3), (0, 3), (0, 7))));
        let lines = lines_of("   foo");
        assert_eq!(aw_range(&lines, 0, 0), Some(around((0, 0), (0, 0), (0, 6))));
        let lines = lines_of("  foo  ");
        assert_eq!(aw_range(&lines, 0, 0), Some(around((0, 0), (0, 0), (0, 5))));
    }

    #[test]
    fn aw_takes_word_plus_trailing_space() {
        let lines = lines_of("foo bar");
        assert_eq!(aw_range(&lines, 0, 0), Some(around((0, 0), (0, 0), (0, 4))));
        let lines = lines_of("foo   bar");
        assert_eq!(aw_range(&lines, 0, 0), Some(around((0, 0), (0, 0), (0, 6))));
        let lines = lines_of("foo, bar");
        assert_eq!(aw_range(&lines, 0, 3), Some(around((0, 3), (0, 3), (0, 5))));
    }

    #[test]
    fn aw_upgrades_to_lines_over_blank() {
        let lines = lines_of("foo\n   \nbar");
        let span = aw_range(&lines, 1, 0).expect("span");
        assert_eq!(
            (span.anchor, span.start, span.end),
            ((1, 0), (1, 0), (2, 3))
        );
        assert!(span.spans_lines(&lines));
        let lines = lines_of("foo   \nbar");
        let span = aw_range(&lines, 0, 3).expect("span");
        assert!(!span.spans_lines(&lines));
    }

    #[test]
    fn aw_without_following_word_is_none() {
        let lines = lines_of("foo   ");
        assert_eq!(aw_range(&lines, 0, 4), None);
        let lines = lines_of("foo\n\n   ");
        assert_eq!(aw_range(&lines, 1, 0), None);
        let lines = lines_of("");
        assert_eq!(aw_range(&lines, 0, 0), None);
    }

    #[test]
    fn aw_without_spaces_takes_word() {
        let lines = lines_of("foo");
        assert_eq!(aw_range(&lines, 0, 0), Some(around((0, 0), (0, 0), (0, 3))));
        let lines = lines_of("foo,");
        assert_eq!(aw_range(&lines, 0, 3), Some(around((0, 3), (0, 3), (0, 4))));
        let lines = lines_of("xx\nbar\nbaz");
        assert_eq!(aw_range(&lines, 1, 0), Some(around((1, 0), (1, 0), (1, 3))));
    }

    #[test]
    fn iw_empty_line_is_none() {
        let lines = lines_of("");
        assert_eq!(iw_range(&lines, 0, 0), None);
        let lines = lines_of("\nfoo");
        assert_eq!(iw_range(&lines, 0, 0), None);
    }

    #[test]
    fn iw_selects_punct_run() {
        let lines = lines_of("foo, bar");
        assert_eq!(iw_range(&lines, 0, 3), Some(around((0, 3), (0, 3), (0, 4))));
        let lines = lines_of("foo...bar");
        assert_eq!(iw_range(&lines, 0, 4), Some(around((0, 3), (0, 3), (0, 6))));
    }

    #[test]
    fn iw_selects_space_run() {
        let lines = lines_of("foo bar");
        assert_eq!(iw_range(&lines, 0, 3), Some(around((0, 3), (0, 3), (0, 4))));
        let lines = lines_of("foo   bar");
        assert_eq!(iw_range(&lines, 0, 4), Some(around((0, 3), (0, 3), (0, 6))));
    }

    #[test]
    fn iw_selects_word_run() {
        let lines = lines_of("foo bar");
        assert_eq!(iw_range(&lines, 0, 0), Some(around((0, 0), (0, 0), (0, 3))));
        assert_eq!(iw_range(&lines, 0, 1), Some(around((0, 0), (0, 0), (0, 3))));
        assert_eq!(iw_range(&lines, 0, 5), Some(around((0, 4), (0, 4), (0, 7))));
    }

    #[test]
    fn iw_stays_on_single_line() {
        let lines = lines_of("xx\n   \nbar");
        assert_eq!(iw_range(&lines, 1, 1), Some(around((1, 0), (1, 0), (1, 3))));
    }
}
