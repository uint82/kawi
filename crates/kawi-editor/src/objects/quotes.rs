use kawi_buffer::Buffer;

use crate::words::WordSpan;

pub fn quote_range(
    buffer: &Buffer,
    row: usize,
    col: usize,
    quote: char,
    inner: bool,
    count: usize,
) -> Option<WordSpan> {
    let line: Vec<char> = buffer.line_cow(row)?.chars().collect();
    let (open, close) = find_quote_pair(&line, col, quote)?;
    if !inner {
        let (start, end) = include_quote_spaces(&line, open, close);
        return Some(WordSpan::Charwise {
            start: (row, start),
            end: (row, end),
        });
    }
    if count < 2 {
        if open + 1 >= close {
            return Some(WordSpan::Empty { at: (row, close) });
        }
        return Some(WordSpan::Charwise {
            start: (row, open + 1),
            end: (row, close - 1),
        });
    }
    Some(WordSpan::Charwise {
        start: (row, open),
        end: (row, close),
    })
}

fn find_quote_pair(line: &[char], col: usize, quote: char) -> Option<(usize, usize)> {
    if line.get(col) == Some(&quote) {
        return scan_quote_pairs(line, col, quote);
    }
    let open = find_prev_quote(line, col, quote);
    if line.get(open) != Some(&quote) {
        let open = find_next_quote(line, open, quote)?;
        let close = find_next_quote_escaped(line, open + 1, quote)?;
        return Some((open, close));
    }
    let close = find_next_quote_escaped(line, open + 1, quote)?;
    Some((open, close))
}

fn scan_quote_pairs(line: &[char], first: usize, quote: char) -> Option<(usize, usize)> {
    let mut open = 0;
    loop {
        open = find_next_quote(line, open, quote)?;
        if open > first {
            return None;
        }
        let close = find_next_quote_escaped(line, open + 1, quote)?;
        if open <= first && first <= close {
            return Some((open, close));
        }
        open = close + 1;
    }
}

fn find_next_quote(line: &[char], col: usize, quote: char) -> Option<usize> {
    let mut col = col;
    while col < line.len() {
        if line[col] == quote {
            return Some(col);
        }
        col += 1;
    }
    None
}

fn find_next_quote_escaped(line: &[char], mut col: usize, quote: char) -> Option<usize> {
    loop {
        match line.get(col) {
            None => return None,
            Some(&'\\') => {
                col += 1;
                line.get(col)?;
            }
            Some(ch) if *ch == quote => return Some(col),
            _ => {}
        }
        col += 1;
    }
}

fn find_prev_quote(line: &[char], col_start: usize, quote: char) -> usize {
    let mut col = col_start;
    while col > 0 {
        col -= 1;
        let mut skipped = 0;
        while col > skipped && line.get(col - skipped - 1) == Some(&'\\') {
            skipped += 1;
        }
        if skipped & 1 == 1 {
            col -= skipped;
        } else if line.get(col) == Some(&quote) {
            break;
        }
    }
    col
}

fn include_quote_spaces(line: &[char], open: usize, close: usize) -> (usize, usize) {
    let mut start = open;
    let mut end = close;
    if line.get(end + 1).is_some_and(|ch| ch.is_whitespace()) {
        while line.get(end + 1).is_some_and(|ch| ch.is_whitespace()) {
            end += 1;
        }
    } else {
        while start > 0 && line.get(start - 1).is_some_and(|ch| ch.is_whitespace()) {
            start -= 1;
        }
    }
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_buffer::Buffer;

    fn lines_of(text: &str) -> Buffer {
        Buffer::from_string(text)
    }

    fn charwise(start: (usize, usize), end: (usize, usize)) -> WordSpan {
        WordSpan::Charwise { start, end }
    }

    fn single(
        buffer: &Buffer,
        row: usize,
        col: usize,
        inner: bool,
        count: usize,
    ) -> Option<WordSpan> {
        quote_range(buffer, row, col, '\'', inner, count)
    }

    #[test]
    fn inner_deletes_content() {
        let lines = lines_of("a 'bc' d");
        assert_eq!(
            single(&lines, 0, 4, true, 1),
            Some(charwise((0, 3), (0, 4)))
        );
    }

    #[test]
    fn around_extends_trailing_space() {
        let lines = lines_of("a 'bc' d");
        assert_eq!(
            single(&lines, 0, 4, false, 1),
            Some(charwise((0, 2), (0, 6)))
        );
    }

    #[test]
    fn around_extends_leading_space() {
        let lines = lines_of("a 'bc'd");
        assert_eq!(
            single(&lines, 0, 4, false, 1),
            Some(charwise((0, 1), (0, 5)))
        );
    }

    #[test]
    fn inner_on_opening_quote() {
        let lines = lines_of("a 'bc' d");
        assert_eq!(
            single(&lines, 0, 2, true, 1),
            Some(charwise((0, 3), (0, 4)))
        );
    }

    #[test]
    fn inner_on_closing_quote() {
        let lines = lines_of("a 'bc' d");
        assert_eq!(
            single(&lines, 0, 5, true, 1),
            Some(charwise((0, 3), (0, 4)))
        );
    }

    #[test]
    fn inner_before_pair() {
        let lines = lines_of("a 'bc' d");
        assert_eq!(
            single(&lines, 0, 0, true, 1),
            Some(charwise((0, 3), (0, 4)))
        );
    }

    #[test]
    fn inner_after_pair_is_none() {
        let lines = lines_of("a 'bc' d");
        assert_eq!(single(&lines, 0, 7, true, 1), None);
    }

    #[test]
    fn inner_respects_escapes() {
        let lines = lines_of("a 'b\\'c' d");
        assert_eq!(
            single(&lines, 0, 4, true, 1),
            Some(charwise((0, 3), (0, 6)))
        );
    }

    #[test]
    fn inner_adjacent_is_empty_at_close() {
        let lines = lines_of("a '' b");
        assert_eq!(
            single(&lines, 0, 2, true, 1),
            Some(WordSpan::Empty { at: (0, 3) })
        );
    }

    #[test]
    fn inner_after_empty_aborts() {
        let lines = lines_of("a '' b");
        assert_eq!(single(&lines, 0, 4, true, 1), None);
    }

    #[test]
    fn around_empty_aborts() {
        let lines = lines_of("a '' b");
        assert_eq!(single(&lines, 0, 4, false, 1), None);
    }

    #[test]
    fn inner_second_pair() {
        let lines = lines_of("'a' 'b'");
        assert_eq!(
            single(&lines, 0, 4, true, 1),
            Some(charwise((0, 5), (0, 5)))
        );
    }

    #[test]
    fn inner_between_pairs_takes_gap() {
        let lines = lines_of("'a' 'b'");
        assert_eq!(
            single(&lines, 0, 3, true, 1),
            Some(charwise((0, 3), (0, 3)))
        );
    }

    #[test]
    fn inner_with_count_includes_quotes() {
        let lines = lines_of("a 'bc' d");
        assert_eq!(
            single(&lines, 0, 4, true, 2),
            Some(charwise((0, 2), (0, 5)))
        );
    }

    #[test]
    fn double_quotes_match() {
        let lines = lines_of("a \"bc\" d");
        assert_eq!(
            quote_range(&lines, 0, 4, '"', true, 1),
            Some(charwise((0, 3), (0, 4)))
        );
        assert_eq!(
            quote_range(&lines, 0, 4, '"', false, 1),
            Some(charwise((0, 2), (0, 6)))
        );
    }

    #[test]
    fn backticks_match() {
        let lines = lines_of("a `bc` d");
        assert_eq!(
            quote_range(&lines, 0, 4, '`', true, 1),
            Some(charwise((0, 3), (0, 4)))
        );
    }

    #[test]
    fn inner_multiline_aborts() {
        let lines = lines_of("a 'b\nc' d");
        assert_eq!(single(&lines, 1, 0, true, 1), None);
    }
}
