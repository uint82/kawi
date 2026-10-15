use kawi_buffer::Buffer;

use crate::words::{char_at, dec_pos, inc_pos, resolve_exclusive, resolve_inclusive, WordSpan};

pub fn bracket_range(
    buffer: &Buffer,
    row: usize,
    col: usize,
    open: char,
    close: char,
    inner: bool,
    count: usize,
) -> Option<WordSpan> {
    let (mut cursor_row, mut cursor_col) = (row, col);
    if char_at(buffer, cursor_row, cursor_col) == Some(open) {
        cursor_col += 1;
    }
    let mut start = None;
    let mut backward_ok = true;
    for _ in 0..count {
        match find_open_backward(buffer, cursor_row, cursor_col, open, close) {
            Some(found) => {
                cursor_row = found.0;
                cursor_col = found.1;
                start = Some(found);
            }
            None => {
                backward_ok = false;
                break;
            }
        }
    }
    let start = match (backward_ok, start) {
        (true, Some(found)) => found,
        _ => {
            let mut cursor = (row, col);
            let mut found = None;
            for _ in 0..count {
                let next = find_open_forward(buffer, cursor.0, cursor.1, open)?;
                cursor = next;
                found = Some(next);
            }
            found?
        }
    };
    let end = find_close_forward(buffer, start.0, start.1, open, close)?;
    if !inner {
        return Some(resolve_inclusive(buffer, start, end, true));
    }
    let (inner_start, inner_end, sol) = exclude_brackets(buffer, start, end);
    if inner_start > inner_end {
        return Some(WordSpan::Empty { at: inner_start });
    }
    if sol {
        return Some(resolve_exclusive(buffer, inner_start, inner_end));
    }
    Some(WordSpan::Charwise {
        start: inner_start,
        end: inner_end,
    })
}

fn exclude_brackets(
    buffer: &Buffer,
    open: (usize, usize),
    close: (usize, usize),
) -> ((usize, usize), (usize, usize), bool) {
    let start = incl_pos(buffer, open.0, open.1);
    let mut end = decl_pos(buffer, close.0, close.1);
    let mut sol = close.1 == 0;
    while strictly_before_nonblank(buffer, end.0, end.1) {
        sol = true;
        let (next, step) = dec_pos(buffer, end.0, end.1);
        end = next;
        if step != 0 {
            break;
        }
    }
    if sol {
        end = incl_pos(buffer, end.0, end.1);
    }
    (start, end, sol)
}

// TODO(objects): skip brackets inside quotes like vim smartmatch.
fn find_open_backward(
    buffer: &Buffer,
    row: usize,
    col: usize,
    open: char,
    close: char,
) -> Option<(usize, usize)> {
    let (mut row, mut col) = (row, col);
    let mut depth = 0;
    loop {
        let (next, step) = dec_pos(buffer, row, col);
        if step == -1 {
            return None;
        }
        row = next.0;
        col = next.1;
        match char_at(buffer, row, col) {
            Some(ch) if ch == close => depth += 1,
            Some(ch) if ch == open => {
                if depth == 0 {
                    return Some((row, col));
                }
                depth -= 1;
            }
            _ => {}
        }
    }
}

fn find_open_forward(
    buffer: &Buffer,
    row: usize,
    col: usize,
    open: char,
) -> Option<(usize, usize)> {
    let (mut row, mut col) = (row, col);
    loop {
        let (next, step) = inc_pos(buffer, row, col);
        if step == -1 {
            return None;
        }
        row = next.0;
        col = next.1;
        if char_at(buffer, row, col) == Some(open) {
            return Some((row, col));
        }
    }
}

fn find_close_forward(
    buffer: &Buffer,
    row: usize,
    col: usize,
    open: char,
    close: char,
) -> Option<(usize, usize)> {
    let (mut row, mut col) = (row, col);
    let mut depth = 0;
    loop {
        let (next, step) = inc_pos(buffer, row, col);
        if step == -1 {
            return None;
        }
        row = next.0;
        col = next.1;
        match char_at(buffer, row, col) {
            Some(ch) if ch == open => depth += 1,
            Some(ch) if ch == close => {
                if depth == 0 {
                    return Some((row, col));
                }
                depth -= 1;
            }
            _ => {}
        }
    }
}

fn incl_pos(buffer: &Buffer, row: usize, col: usize) -> (usize, usize) {
    let (mut pos, step) = inc_pos(buffer, row, col);
    if step >= 1 && pos.1 != 0 {
        (pos, _) = inc_pos(buffer, pos.0, pos.1);
    }
    pos
}

fn decl_pos(buffer: &Buffer, row: usize, col: usize) -> (usize, usize) {
    let (mut pos, step) = dec_pos(buffer, row, col);
    if step == 1 && pos.1 != 0 {
        (pos, _) = dec_pos(buffer, pos.0, pos.1);
    }
    pos
}

fn strictly_before_nonblank(buffer: &Buffer, row: usize, col: usize) -> bool {
    match buffer.line_cow(row) {
        None => false,
        Some(line) => col.saturating_add(1) <= kawi_buffer::first_non_blank(&line),
    }
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

    fn paren(
        buffer: &Buffer,
        row: usize,
        col: usize,
        inner: bool,
        count: usize,
    ) -> Option<WordSpan> {
        bracket_range(buffer, row, col, '(', ')', inner, count)
    }

    #[test]
    fn inner_deletes_content() {
        let lines = lines_of("foo (bar) baz");
        assert_eq!(paren(&lines, 0, 5, true, 1), Some(charwise((0, 5), (0, 7))));
    }

    #[test]
    fn around_deletes_brackets() {
        let lines = lines_of("foo (bar) baz");
        assert_eq!(
            paren(&lines, 0, 5, false, 1),
            Some(charwise((0, 4), (0, 8)))
        );
    }

    #[test]
    fn inner_finds_enclosing_from_outside() {
        let lines = lines_of("foo (bar) baz");
        assert_eq!(paren(&lines, 0, 0, true, 1), Some(charwise((0, 5), (0, 7))));
    }

    #[test]
    fn inner_on_opener_works() {
        let lines = lines_of("foo (bar) baz");
        assert_eq!(paren(&lines, 0, 4, true, 1), Some(charwise((0, 5), (0, 7))));
    }

    #[test]
    fn inner_empty_is_empty_at_inner() {
        let lines = lines_of("foo () bar");
        assert_eq!(
            paren(&lines, 0, 5, true, 1),
            Some(WordSpan::Empty { at: (0, 5) })
        );
    }

    #[test]
    fn around_empty_deletes_parens() {
        let lines = lines_of("foo () bar");
        assert_eq!(
            paren(&lines, 0, 5, false, 1),
            Some(charwise((0, 4), (0, 5)))
        );
    }

    #[test]
    fn inner_selects_innermost() {
        let lines = lines_of("foo (a (b) c) bar");
        assert_eq!(paren(&lines, 0, 8, true, 1), Some(charwise((0, 8), (0, 8))));
    }

    #[test]
    fn inner_selects_outer_from_between() {
        let lines = lines_of("foo (a (b) c) bar");
        assert_eq!(
            paren(&lines, 0, 5, true, 1),
            Some(charwise((0, 5), (0, 11)))
        );
    }

    #[test]
    fn around_selects_inner_pair() {
        let lines = lines_of("foo (a (b) c) bar");
        assert_eq!(
            paren(&lines, 0, 8, false, 1),
            Some(charwise((0, 7), (0, 9)))
        );
    }

    #[test]
    fn unmatched_open_is_none() {
        let lines = lines_of("foo (bar baz");
        assert_eq!(paren(&lines, 0, 5, true, 1), None);
        assert_eq!(paren(&lines, 0, 5, false, 1), None);
    }

    #[test]
    fn close_bracket_is_none() {
        let lines = lines_of("foo (bar) baz");
        assert_eq!(paren(&lines, 0, 9, true, 1), None);
        assert_eq!(paren(&lines, 0, 9, false, 1), None);
    }

    #[test]
    fn after_pair_is_none() {
        let lines = lines_of("foo (bar) baz");
        assert_eq!(paren(&lines, 0, 12, true, 1), None);
    }

    #[test]
    fn inner_between_pairs_takes_second() {
        let lines = lines_of("(a) (b)");
        assert_eq!(paren(&lines, 0, 3, true, 1), Some(charwise((0, 5), (0, 5))));
    }

    #[test]
    fn around_between_pairs_takes_second() {
        let lines = lines_of("(a) (b)");
        assert_eq!(
            paren(&lines, 0, 3, false, 1),
            Some(charwise((0, 4), (0, 6)))
        );
    }

    #[test]
    fn inner_multiline_removes_middle_line() {
        let lines = lines_of("foo (\nbar\n) baz");
        assert_eq!(
            paren(&lines, 1, 0, true, 1),
            Some(WordSpan::Linewise { first: 1, last: 1 })
        );
    }

    #[test]
    fn around_multiline_is_charwise() {
        let lines = lines_of("foo (\nbar\n) baz");
        assert_eq!(
            paren(&lines, 1, 0, false, 1),
            Some(charwise((0, 4), (2, 0)))
        );
    }

    #[test]
    fn around_upgrades_over_blank_tail() {
        let lines = lines_of("(a\nb)   ");
        assert_eq!(
            paren(&lines, 0, 0, false, 1),
            Some(WordSpan::Linewise { first: 0, last: 1 })
        );
    }

    #[test]
    fn inner_indented_closer() {
        let lines = lines_of("(a\n     )");
        assert_eq!(paren(&lines, 1, 0, true, 1), Some(charwise((0, 1), (0, 1))));
    }

    #[test]
    fn inner_counts_outward() {
        let lines = lines_of("foo (a (b) c) bar");
        assert_eq!(
            paren(&lines, 0, 8, true, 2),
            Some(charwise((0, 5), (0, 11)))
        );
    }

    #[test]
    fn inner_count_without_outer_is_none() {
        let lines = lines_of("(a) (b)");
        assert_eq!(paren(&lines, 0, 1, true, 2), None);
    }

    #[test]
    fn braces_match_like_parens() {
        let lines = lines_of("foo {bar} baz");
        assert_eq!(
            bracket_range(&lines, 0, 5, '{', '}', true, 1),
            Some(charwise((0, 5), (0, 7)))
        );
        assert_eq!(
            bracket_range(&lines, 0, 5, '{', '}', false, 1),
            Some(charwise((0, 4), (0, 8)))
        );
    }

    #[test]
    fn square_brackets_match_like_parens() {
        let lines = lines_of("foo [bar] baz");
        assert_eq!(
            bracket_range(&lines, 0, 5, '[', ']', true, 1),
            Some(charwise((0, 5), (0, 7)))
        );
        assert_eq!(
            bracket_range(&lines, 0, 5, '[', ']', false, 1),
            Some(charwise((0, 4), (0, 8)))
        );
    }

    #[test]
    fn angles_match_like_parens() {
        let lines = lines_of("a <b> c");
        assert_eq!(
            bracket_range(&lines, 0, 3, '<', '>', true, 1),
            Some(charwise((0, 3), (0, 3)))
        );
        assert_eq!(
            bracket_range(&lines, 0, 3, '<', '>', false, 1),
            Some(charwise((0, 2), (0, 4)))
        );
    }
}
