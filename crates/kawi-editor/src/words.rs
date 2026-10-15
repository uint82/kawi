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

fn remainder_blank(buffer: &Buffer, end: (usize, usize)) -> bool {
    match buffer.line_cow(end.0) {
        None => true,
        Some(line) => line.chars().skip(end.1).all(|ch| ch.is_whitespace()),
    }
}

fn start_blank(buffer: &Buffer, start: (usize, usize)) -> bool {
    match buffer.line_cow(start.0) {
        None => true,
        Some(line) => line.chars().take(start.1).all(|ch| ch.is_whitespace()),
    }
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

    fn around(anchor: (usize, usize), start: (usize, usize), end: (usize, usize)) -> AroundSpan {
        AroundSpan { anchor, start, end }
    }

    #[test]
    fn iw_selects_word_run() {
        let lines = lines_of("foo bar");
        assert_eq!(iw_range(&lines, 0, 0), Some(around((0, 0), (0, 0), (0, 3))));
        assert_eq!(iw_range(&lines, 0, 1), Some(around((0, 0), (0, 0), (0, 3))));
        assert_eq!(iw_range(&lines, 0, 5), Some(around((0, 4), (0, 4), (0, 7))));
    }

    #[test]
    fn iw_selects_space_run() {
        let lines = lines_of("foo bar");
        assert_eq!(iw_range(&lines, 0, 3), Some(around((0, 3), (0, 3), (0, 4))));
        let lines = lines_of("foo   bar");
        assert_eq!(iw_range(&lines, 0, 4), Some(around((0, 3), (0, 3), (0, 6))));
    }

    #[test]
    fn iw_selects_punct_run() {
        let lines = lines_of("foo, bar");
        assert_eq!(iw_range(&lines, 0, 3), Some(around((0, 3), (0, 3), (0, 4))));
        let lines = lines_of("foo...bar");
        assert_eq!(iw_range(&lines, 0, 4), Some(around((0, 3), (0, 3), (0, 6))));
    }

    #[test]
    fn iw_empty_line_is_none() {
        let lines = lines_of("");
        assert_eq!(iw_range(&lines, 0, 0), None);
        let lines = lines_of("\nfoo");
        assert_eq!(iw_range(&lines, 0, 0), None);
    }

    #[test]
    fn iw_stays_on_single_line() {
        let lines = lines_of("xx\n   \nbar");
        assert_eq!(iw_range(&lines, 1, 1), Some(around((1, 0), (1, 0), (1, 3))));
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
    fn aw_at_end_takes_leading_space() {
        let lines = lines_of("foo bar");
        assert_eq!(aw_range(&lines, 0, 4), Some(around((0, 3), (0, 3), (0, 7))));
        let lines = lines_of("xx foo,bar");
        assert_eq!(aw_range(&lines, 0, 3), Some(around((0, 2), (0, 2), (0, 6))));
        let lines = lines_of("foo,bar");
        assert_eq!(aw_range(&lines, 0, 0), Some(around((0, 0), (0, 0), (0, 3))));
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
    fn aw_keeps_indent_at_line_start() {
        let lines = lines_of("   foo\nbar");
        assert_eq!(aw_range(&lines, 0, 3), Some(around((0, 3), (0, 3), (0, 6))));
        let lines = lines_of("   foo");
        assert_eq!(aw_range(&lines, 0, 3), Some(around((0, 3), (0, 3), (0, 6))));
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
    fn aw_without_following_word_is_none() {
        let lines = lines_of("foo   ");
        assert_eq!(aw_range(&lines, 0, 4), None);
        let lines = lines_of("foo\n\n   ");
        assert_eq!(aw_range(&lines, 1, 0), None);
        let lines = lines_of("");
        assert_eq!(aw_range(&lines, 0, 0), None);
    }

    #[test]
    fn aw_crosses_lines_from_space() {
        let lines = lines_of("foo   \nbar");
        assert_eq!(aw_range(&lines, 0, 4), Some(around((0, 4), (0, 3), (1, 3))));
        let lines = lines_of("foo   \n\nbar");
        assert_eq!(aw_range(&lines, 0, 4), Some(around((0, 4), (0, 3), (1, 0))));
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
    fn aw_on_empty_line_stops_at_empty() {
        let lines = lines_of("\n\nfoo");
        assert_eq!(aw_range(&lines, 0, 0), Some(around((0, 0), (0, 0), (1, 0))));
        let lines = lines_of("\n\n\nfoo");
        assert_eq!(aw_range(&lines, 0, 0), Some(around((0, 0), (0, 0), (1, 0))));
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
}
