use kawi_buffer::Buffer;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum WordClass {
    Word,
    Punct,
    Space,
}

pub fn next_word_start(buffer: &Buffer, row: usize, col: usize, count: usize) -> (usize, usize) {
    fwd_word(buffer, row, col, false, count)
}

pub fn prev_word_start(buffer: &Buffer, row: usize, col: usize, count: usize) -> (usize, usize) {
    bck_word(buffer, row, col, false, count)
}

pub fn next_word_end(buffer: &Buffer, row: usize, col: usize, count: usize) -> (usize, usize) {
    end_word(buffer, row, col, false, false, count)
}

pub fn dw_range(buffer: &Buffer, row: usize, col: usize, count: usize) -> WordSpan {
    resolve_exclusive(
        buffer,
        (row, col),
        fwd_word_raw(buffer, row, col, true, count),
    )
}

pub fn db_range(buffer: &Buffer, row: usize, col: usize, count: usize) -> WordSpan {
    resolve_exclusive(buffer, (row, col), bck_word(buffer, row, col, false, count))
}

pub fn cw_range(buffer: &Buffer, row: usize, col: usize, count: usize) -> WordSpan {
    resolve_inclusive(
        buffer,
        (row, col),
        end_word_raw(buffer, row, col, false, false, count),
        true,
    )
}

pub fn ce_range(buffer: &Buffer, row: usize, col: usize, count: usize) -> WordSpan {
    resolve_inclusive(
        buffer,
        (row, col),
        end_word_raw(buffer, row, col, false, false, count),
        false,
    )
}

pub fn cw_change_range(buffer: &Buffer, row: usize, col: usize, count: usize) -> WordSpan {
    match char_at(buffer, row, col).map(class_of) {
        Some(WordClass::Space) | None => dw_range(buffer, row, col, count),
        _ => {
            let end = end_word_raw(buffer, row, col, true, false, count);
            let (end, _) = adjust_end(buffer, (row, col), end, true);
            let (start, end) = ordered_span((row, col), end);
            WordSpan::Charwise { start, end }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordSpan {
    Charwise {
        start: (usize, usize),
        end: (usize, usize),
    },
    Linewise {
        first: usize,
        last: usize,
    },
    Empty {
        at: (usize, usize),
    },
}

pub(crate) fn resolve_exclusive(
    buffer: &Buffer,
    start: (usize, usize),
    raw_end: (usize, usize),
) -> WordSpan {
    let (end, inclusive) = adjust_end(buffer, start, raw_end, false);
    let (start, end) = ordered_span(start, end);
    if inclusive {
        return WordSpan::Charwise { start, end };
    }
    if start == end {
        return WordSpan::Empty { at: start };
    }
    if end.1 == 0 && end.0 > start.0 {
        return shift_exclusive_end(buffer, start, end);
    }
    match prev_char_pos(buffer, end.0, end.1) {
        Some(before) => WordSpan::Charwise { start, end: before },
        None => WordSpan::Empty { at: start },
    }
}

fn shift_exclusive_end(buffer: &Buffer, start: (usize, usize), end: (usize, usize)) -> WordSpan {
    if start_blank(buffer, start) {
        return WordSpan::Linewise {
            first: start.0,
            last: end.0.saturating_sub(1),
        };
    }
    let prev = end.0.saturating_sub(1);
    if line_len(buffer, prev) > 0 {
        return WordSpan::Charwise {
            start,
            end: (prev, line_len(buffer, prev).saturating_sub(1)),
        };
    }
    match prev_char_pos(buffer, prev, 0) {
        Some(before) => WordSpan::Charwise { start, end: before },
        None => WordSpan::Empty { at: start },
    }
}

pub(crate) fn resolve_inclusive(
    buffer: &Buffer,
    start: (usize, usize),
    raw_end: (usize, usize),
    upgrade: bool,
) -> WordSpan {
    let (end, _) = adjust_end(buffer, start, raw_end, true);
    let (start, end) = ordered_span(start, end);
    if start == end && line_is_empty(buffer, start.0) {
        return WordSpan::Empty { at: start };
    }
    if upgrade && end.0 > start.0 && tail_blank(buffer, end) && start_blank(buffer, start) {
        return WordSpan::Linewise {
            first: start.0,
            last: end.0,
        };
    }
    WordSpan::Charwise { start, end }
}

fn adjust_end(
    buffer: &Buffer,
    start: (usize, usize),
    end: (usize, usize),
    inclusive: bool,
) -> ((usize, usize), bool) {
    if end > start && end.1 > 0 && end.1 == line_len(buffer, end.0) {
        return ((end.0, end.1.saturating_sub(1)), true);
    }
    (end, inclusive)
}

fn fwd_word(buffer: &Buffer, row: usize, col: usize, eol: bool, count: usize) -> (usize, usize) {
    let start = (row, col);
    adjust_end(
        buffer,
        start,
        fwd_word_raw(buffer, row, col, eol, count),
        false,
    )
    .0
}

fn fwd_word_raw(
    buffer: &Buffer,
    row: usize,
    col: usize,
    eol: bool,
    count: usize,
) -> (usize, usize) {
    let last_line = row.saturating_add(1) == buffer.line_count();
    let mut pos = (row, col);
    for remaining in (0..count).rev() {
        let last = remaining == 0;
        let sclass = class_at(buffer, pos.0, pos.1);
        let (next, step) = inc_pos(buffer, pos.0, pos.1);
        pos = next;
        if step == -1 || (step >= 1 && last_line) {
            return pos;
        }
        if step >= 1 && eol && last {
            return pos;
        }
        if sclass != WordClass::Space {
            while class_at(buffer, pos.0, pos.1) == sclass {
                let (next, step) = inc_pos(buffer, pos.0, pos.1);
                pos = next;
                if step == -1 || (step >= 1 && eol && last) {
                    return pos;
                }
            }
        }
        while class_at(buffer, pos.0, pos.1) == WordClass::Space {
            if pos.1 == 0 && line_is_empty(buffer, pos.0) {
                break;
            }
            let (next, step) = inc_pos(buffer, pos.0, pos.1);
            pos = next;
            if step == -1 || (step >= 1 && eol && last) {
                return pos;
            }
        }
    }
    pos
}

fn end_word(
    buffer: &Buffer,
    row: usize,
    col: usize,
    stop: bool,
    empty: bool,
    count: usize,
) -> (usize, usize) {
    let start = (row, col);
    adjust_end(
        buffer,
        start,
        end_word_raw(buffer, row, col, stop, empty, count),
        true,
    )
    .0
}

fn end_word_raw(
    buffer: &Buffer,
    row: usize,
    col: usize,
    stop: bool,
    empty: bool,
    count: usize,
) -> (usize, usize) {
    let mut pos = (row, col);
    let mut stop = stop;
    for _ in 0..count {
        let iter_start = pos;
        let settled = !stop;
        let sclass = class_at(buffer, pos.0, pos.1);
        let (next, step) = inc_pos(buffer, pos.0, pos.1);
        pos = next;
        if step == -1 {
            return pos;
        }
        if class_at(buffer, pos.0, pos.1) == sclass && sclass != WordClass::Space {
            if skip_same_class(buffer, &mut pos, true) {
                return pos;
            }
        } else if !stop || sclass == WordClass::Space {
            while class_at(buffer, pos.0, pos.1) == WordClass::Space {
                if empty && pos.1 == 0 && line_is_empty(buffer, pos.0) {
                    break;
                }
                let (next, step) = inc_pos(buffer, pos.0, pos.1);
                pos = next;
                if step == -1 {
                    return pos;
                }
            }
            if skip_same_class(buffer, &mut pos, true) {
                return pos;
            }
        }
        let (back, _) = dec_pos(buffer, pos.0, pos.1);
        pos = back;
        stop = false;
        if settled && pos == iter_start {
            break;
        }
    }
    pos
}

fn bck_word(buffer: &Buffer, row: usize, col: usize, stop: bool, count: usize) -> (usize, usize) {
    let mut pos = (row, col);
    let mut stop = stop;
    for _ in 0..count {
        let iter_start = pos;
        let settled = !stop;
        let sclass = class_at(buffer, pos.0, pos.1);
        let (next, step) = dec_pos(buffer, pos.0, pos.1);
        pos = next;
        if step == -1 {
            return pos;
        }
        let mut landed_empty = false;
        if !stop || sclass == class_at(buffer, pos.0, pos.1) || sclass == WordClass::Space {
            while class_at(buffer, pos.0, pos.1) == WordClass::Space {
                if pos.1 == 0 && line_is_empty(buffer, pos.0) {
                    landed_empty = true;
                    break;
                }
                let (next, step) = dec_pos(buffer, pos.0, pos.1);
                pos = next;
                if step == -1 {
                    return pos;
                }
            }
            if !landed_empty && skip_same_class(buffer, &mut pos, false) {
                return pos;
            }
        }
        if !landed_empty {
            let (next, _) = inc_pos(buffer, pos.0, pos.1);
            pos = next;
        }
        stop = false;
        if settled && pos == iter_start {
            break;
        }
    }
    pos
}

pub fn extend_word_head(
    buffer: &Buffer,
    anchor: (usize, usize),
    head: (usize, usize),
    inner: bool,
    steps: usize,
) -> (usize, usize) {
    let mut pos = head;
    for _ in 0..steps {
        let next = extend_word_step(buffer, anchor, pos, inner);
        if next == pos {
            break;
        }
        pos = next;
    }
    pos
}

fn extend_word_step(
    buffer: &Buffer,
    anchor: (usize, usize),
    head: (usize, usize),
    inner: bool,
) -> (usize, usize) {
    if head < anchor {
        let (mut pos, step) = dec_pos(buffer, head.0, head.1);
        if step == -1 {
            return head;
        }
        if !inner != (class_at(buffer, pos.0, pos.1) != WordClass::Space) {
            return bck_word(buffer, pos.0, pos.1, true, 1);
        }
        pos = bckend_word(buffer, pos.0, pos.1, 1);
        let (next, _) = inc_pos(buffer, pos.0, pos.1);
        return next;
    }
    let (mut pos, step) = inc_pos(buffer, head.0, head.1);
    if step == -1 {
        return head;
    }
    if !inner != (class_at(buffer, pos.0, pos.1) == WordClass::Space) {
        pos = fwd_word_raw(buffer, pos.0, pos.1, true, 1);
        let (back, _) = dec_pos(buffer, pos.0, pos.1);
        return back;
    }
    end_word_raw(buffer, pos.0, pos.1, true, true, 1)
}

pub(crate) fn bckend_word(buffer: &Buffer, row: usize, col: usize, count: usize) -> (usize, usize) {
    let mut pos = (row, col);
    for _ in 0..count {
        let sclass = class_at(buffer, pos.0, pos.1);
        let (next, step) = dec_pos(buffer, pos.0, pos.1);
        if step == -1 {
            return pos;
        }
        pos = next;
        if step == 1 {
            return pos;
        }
        if sclass != WordClass::Space {
            while class_at(buffer, pos.0, pos.1) == sclass {
                let (next, step) = dec_pos(buffer, pos.0, pos.1);
                pos = next;
                if step == -1 || step == 1 {
                    return pos;
                }
            }
        }
        while class_at(buffer, pos.0, pos.1) == WordClass::Space {
            if pos.1 == 0 && line_is_empty(buffer, pos.0) {
                break;
            }
            let (next, step) = dec_pos(buffer, pos.0, pos.1);
            pos = next;
            if step == -1 || step == 1 {
                return pos;
            }
        }
    }
    pos
}

fn skip_same_class(buffer: &Buffer, pos: &mut (usize, usize), forward: bool) -> bool {
    let class = class_at(buffer, pos.0, pos.1);
    loop {
        if class_at(buffer, pos.0, pos.1) != class {
            return false;
        }
        let (next, step) = if forward {
            inc_pos(buffer, pos.0, pos.1)
        } else {
            dec_pos(buffer, pos.0, pos.1)
        };
        *pos = next;
        if step == -1 {
            return true;
        }
    }
}

pub(crate) fn inc_pos(buffer: &Buffer, row: usize, col: usize) -> ((usize, usize), i32) {
    let len = line_len(buffer, row);
    let col = col.min(len);
    if col < len {
        let next = col.saturating_add(1);
        let step = if next < len { 0 } else { 2 };
        return ((row, next), step);
    }
    if row.saturating_add(1) < buffer.line_count() {
        return ((row.saturating_add(1), 0), 1);
    }
    ((row, col), -1)
}

pub(crate) fn dec_pos(buffer: &Buffer, row: usize, col: usize) -> ((usize, usize), i32) {
    if col > 0 {
        return ((row, col.saturating_sub(1)), 0);
    }
    if row > 0 {
        let prev = row.saturating_sub(1);
        return ((prev, line_len(buffer, prev)), 1);
    }
    ((row, col), -1)
}

fn class_at(buffer: &Buffer, row: usize, col: usize) -> WordClass {
    match char_at(buffer, row, col) {
        Some(ch) => class_of(ch),
        None => WordClass::Space,
    }
}

fn line_is_empty(buffer: &Buffer, row: usize) -> bool {
    line_len(buffer, row) == 0
}

fn tail_blank(buffer: &Buffer, end: (usize, usize)) -> bool {
    match buffer.line_cow(end.0) {
        None => true,
        Some(line) => line
            .chars()
            .skip(end.1.saturating_add(1))
            .all(|ch| ch.is_whitespace()),
    }
}

fn ordered_span(first: (usize, usize), second: (usize, usize)) -> ((usize, usize), (usize, usize)) {
    if first <= second {
        (first, second)
    } else {
        (second, first)
    }
}

pub(crate) fn remainder_blank(buffer: &Buffer, end: (usize, usize)) -> bool {
    match buffer.line_cow(end.0) {
        None => true,
        Some(line) => line.chars().skip(end.1).all(|ch| ch.is_whitespace()),
    }
}

pub(crate) fn start_blank(buffer: &Buffer, start: (usize, usize)) -> bool {
    match buffer.line_cow(start.0) {
        None => true,
        Some(line) => line.chars().take(start.1).all(|ch| ch.is_whitespace()),
    }
}

pub(crate) fn prev_char_pos(buffer: &Buffer, row: usize, col: usize) -> Option<(usize, usize)> {
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

pub(crate) fn char_at(buffer: &Buffer, row: usize, col: usize) -> Option<char> {
    buffer.line_cow(row)?.chars().nth(col)
}

pub(crate) fn line_len(buffer: &Buffer, row: usize) -> usize {
    buffer.line_len(row)
}

pub(crate) fn class_of(ch: char) -> WordClass {
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

    fn charwise(start: (usize, usize), end: (usize, usize)) -> WordSpan {
        WordSpan::Charwise { start, end }
    }

    #[test]
    fn word_motions_take_count() {
        let lines = lines_of("foo bar baz");
        assert_eq!(next_word_start(&lines, 0, 0, 2), (0, 8));
        assert_eq!(prev_word_start(&lines, 0, 8, 2), (0, 0));
        assert_eq!(next_word_end(&lines, 0, 0, 2), (0, 6));
    }

    #[test]
    fn word_motions_take_long_count() {
        let lines = lines_of("a b c d e f g h i j k");
        assert_eq!(next_word_start(&lines, 0, 0, 10), (0, 20));
    }

    #[test]
    fn dw_takes_count() {
        let lines = lines_of("foo bar baz");
        assert_eq!(dw_range(&lines, 0, 0, 2), charwise((0, 0), (0, 7)));
    }

    #[test]
    fn dw_multiplies_counts() {
        let lines = lines_of("foo bar qux baz");
        assert_eq!(dw_range(&lines, 0, 0, 4), charwise((0, 0), (0, 14)));
    }

    #[test]
    fn db_takes_count() {
        let lines = lines_of("foo bar baz");
        assert_eq!(db_range(&lines, 0, 10, 2), charwise((0, 4), (0, 9)));
    }

    #[test]
    fn de_takes_count() {
        let lines = lines_of("foo bar baz");
        assert_eq!(cw_range(&lines, 0, 0, 2), charwise((0, 0), (0, 6)));
        assert_eq!(ce_range(&lines, 0, 0, 2), charwise((0, 0), (0, 6)));
    }

    #[test]
    fn cw_takes_count() {
        let lines = lines_of("foo bar baz");
        assert_eq!(cw_change_range(&lines, 0, 0, 2), charwise((0, 0), (0, 6)));
    }

    #[test]
    fn cw_from_end_takes_count() {
        let lines = lines_of("foo bar");
        assert_eq!(cw_change_range(&lines, 0, 2, 2), charwise((0, 2), (0, 6)));
    }

    #[test]
    fn w_moves_to_next_start() {
        let lines = lines_of("foo bar");
        assert_eq!(next_word_start(&lines, 0, 0, 1), (0, 4));
    }

    #[test]
    fn w_stops_on_punct() {
        let lines = lines_of("foo, bar");
        assert_eq!(next_word_start(&lines, 0, 0, 1), (0, 3));
        assert_eq!(next_word_start(&lines, 0, 3, 1), (0, 5));
    }

    #[test]
    fn w_skips_gaps() {
        let lines = lines_of("foo   bar");
        assert_eq!(next_word_start(&lines, 0, 1, 1), (0, 6));
    }

    #[test]
    fn w_stays_at_last_word() {
        let lines = lines_of("foo");
        assert_eq!(next_word_start(&lines, 0, 2, 1), (0, 2));
    }

    #[test]
    fn w_stops_on_empty_line() {
        let lines = lines_of("foo\n\nbar");
        assert_eq!(next_word_start(&lines, 0, 0, 1), (1, 0));
        assert_eq!(next_word_start(&lines, 1, 0, 1), (2, 0));
    }

    #[test]
    fn b_moves_to_prev_start() {
        let lines = lines_of("foo bar");
        assert_eq!(prev_word_start(&lines, 0, 4, 1), (0, 0));
    }

    #[test]
    fn b_stays_at_first_word() {
        let lines = lines_of("foo");
        assert_eq!(prev_word_start(&lines, 0, 0, 1), (0, 0));
    }

    #[test]
    fn b_stops_on_empty_line() {
        let lines = lines_of("foo\n\nbar");
        assert_eq!(prev_word_start(&lines, 2, 0, 1), (1, 0));
        assert_eq!(prev_word_start(&lines, 1, 0, 1), (0, 0));
    }

    #[test]
    fn b_crosses_lines() {
        let lines = lines_of("foo\nbar");
        assert_eq!(prev_word_start(&lines, 1, 0, 1), (0, 0));
    }

    #[test]
    fn e_moves_to_word_end() {
        let lines = lines_of("foo bar");
        assert_eq!(next_word_end(&lines, 0, 0, 1), (0, 2));
        assert_eq!(next_word_end(&lines, 0, 2, 1), (0, 6));
    }

    #[test]
    fn e_stops_on_punct_end() {
        let lines = lines_of("foo, bar");
        assert_eq!(next_word_end(&lines, 0, 2, 1), (0, 3));
        assert_eq!(next_word_end(&lines, 0, 3, 1), (0, 7));
    }

    #[test]
    fn word_end_from_empty_line() {
        let lines = lines_of("foo\n\nbar");
        assert_eq!(next_word_end(&lines, 1, 0, 1), (2, 2));
    }

    #[test]
    fn e_skips_empty_lines() {
        let lines = lines_of("foo\n\nbar");
        assert_eq!(next_word_end(&lines, 0, 2, 1), (2, 2));
    }

    #[test]
    fn e_stays_at_last_end() {
        let lines = lines_of("foo");
        assert_eq!(next_word_end(&lines, 0, 2, 1), (0, 2));
    }

    #[test]
    fn b_from_indent_returns_to_line_start() {
        let lines = lines_of("  foo");
        assert_eq!(prev_word_start(&lines, 0, 2, 1), (0, 0));
    }

    #[test]
    fn w_advances_to_end_on_last_word() {
        let lines = lines_of("foo");
        assert_eq!(next_word_start(&lines, 0, 0, 1), (0, 2));
        assert_eq!(next_word_start(&lines, 0, 1, 1), (0, 2));
    }

    #[test]
    fn w_moves_to_trailing_space() {
        let lines = lines_of("foo ");
        assert_eq!(next_word_start(&lines, 0, 0, 1), (0, 3));
        assert_eq!(next_word_start(&lines, 0, 2, 1), (0, 3));
    }

    #[test]
    fn w_moves_off_whitespace_line() {
        let lines = lines_of("  ");
        assert_eq!(next_word_start(&lines, 0, 0, 1), (0, 1));
    }

    #[test]
    fn w_moves_to_next_empty_line() {
        let lines = lines_of("\n\n");
        assert_eq!(next_word_start(&lines, 0, 0, 1), (1, 0));
    }

    #[test]
    fn e_moves_off_whitespace_line() {
        let lines = lines_of("  ");
        assert_eq!(next_word_end(&lines, 0, 0, 1), (0, 1));
    }

    #[test]
    fn b_moves_off_whitespace_line() {
        let lines = lines_of("  ");
        assert_eq!(prev_word_start(&lines, 0, 1, 1), (0, 0));
    }

    #[test]
    fn change_word_on_end_stays_on_char() {
        let lines = lines_of("foo bar");
        assert_eq!(
            cw_change_range(&lines, 0, 2, 1),
            WordSpan::Charwise {
                start: (0, 2),
                end: (0, 2)
            }
        );
        assert_eq!(
            cw_change_range(&lines, 0, 0, 1),
            WordSpan::Charwise {
                start: (0, 0),
                end: (0, 2)
            }
        );
    }

    #[test]
    fn change_word_on_space_reaches_next_word() {
        let lines = lines_of("foo   bar");
        assert_eq!(
            cw_change_range(&lines, 0, 4, 1),
            WordSpan::Charwise {
                start: (0, 4),
                end: (0, 5)
            }
        );
    }

    #[test]
    fn change_word_on_empty_changes_line() {
        let lines = lines_of("foo\n\nbar");
        assert_eq!(
            cw_change_range(&lines, 1, 0, 1),
            WordSpan::Linewise { first: 1, last: 1 }
        );
    }

    #[test]
    fn change_word_on_first_empty_changes_line() {
        let lines = lines_of("\nbar");
        assert_eq!(
            cw_change_range(&lines, 0, 0, 1),
            WordSpan::Linewise { first: 0, last: 0 }
        );
    }

    #[test]
    fn change_word_on_word_end_stays_single() {
        let lines = lines_of("foo\nbar");
        assert_eq!(
            cw_change_range(&lines, 0, 2, 1),
            WordSpan::Charwise {
                start: (0, 2),
                end: (0, 2)
            }
        );
    }

    #[test]
    fn dw_deletes_to_next_start() {
        let lines = lines_of("foo bar");
        assert_eq!(dw_range(&lines, 0, 0, 1), charwise((0, 0), (0, 3)));
    }

    #[test]
    fn dw_last_word_deletes_to_end() {
        let lines = lines_of("foo bar");
        assert_eq!(dw_range(&lines, 0, 4, 1), charwise((0, 4), (0, 6)));
    }

    #[test]
    fn dw_space_deletes_spaces() {
        let lines = lines_of("foo   bar");
        assert_eq!(dw_range(&lines, 0, 3, 1), charwise((0, 3), (0, 5)));
    }

    #[test]
    fn dw_last_char_deletes_single() {
        let lines = lines_of("foo");
        assert_eq!(dw_range(&lines, 0, 2, 1), charwise((0, 2), (0, 2)));
    }

    #[test]
    fn dw_empty_file_is_empty() {
        let lines = lines_of("");
        assert_eq!(dw_range(&lines, 0, 0, 1), WordSpan::Empty { at: (0, 0) });
    }

    #[test]
    fn dw_across_lines_stops_before_target() {
        let lines = lines_of("foo\nbar");
        assert_eq!(dw_range(&lines, 0, 0, 1), charwise((0, 0), (0, 2)));
    }

    #[test]
    fn dw_empty_line_keeps_newline() {
        let lines = lines_of("foo\n\nbar");
        assert_eq!(dw_range(&lines, 0, 0, 1), charwise((0, 0), (0, 2)));
    }

    #[test]
    fn dw_on_empty_line_removes_line() {
        let lines = lines_of("foo\n\nbar");
        assert_eq!(
            dw_range(&lines, 1, 0, 1),
            WordSpan::Linewise { first: 1, last: 1 }
        );
    }

    #[test]
    fn dw_on_first_of_two_empties_removes_line() {
        let lines = lines_of("\n\n");
        assert_eq!(
            dw_range(&lines, 0, 0, 1),
            WordSpan::Linewise { first: 0, last: 0 }
        );
    }

    #[test]
    fn dw_on_trailing_space_deletes_space() {
        let lines = lines_of("foo \nbar");
        assert_eq!(dw_range(&lines, 0, 3, 1), charwise((0, 3), (0, 3)));
    }

    #[test]
    fn dw_stops_at_line_end() {
        let lines = lines_of("ab \ncd");
        assert_eq!(dw_range(&lines, 0, 2, 1), charwise((0, 2), (0, 2)));
    }

    #[test]
    fn cw_deletes_to_word_end() {
        let lines = lines_of("foo bar");
        assert_eq!(cw_range(&lines, 0, 0, 1), charwise((0, 0), (0, 2)));
        assert_eq!(cw_range(&lines, 0, 1, 1), charwise((0, 1), (0, 2)));
    }

    #[test]
    fn cw_empty_file_is_empty() {
        let lines = lines_of("");
        assert_eq!(cw_range(&lines, 0, 0, 1), WordSpan::Empty { at: (0, 0) });
    }

    #[test]
    fn de_empty_line_takes_next_word() {
        let lines = lines_of("\nbar");
        assert_eq!(
            cw_range(&lines, 0, 0, 1),
            WordSpan::Linewise { first: 0, last: 1 }
        );
    }

    #[test]
    fn de_two_empties_takes_both() {
        let lines = lines_of("\n\n");
        assert_eq!(
            cw_range(&lines, 0, 0, 1),
            WordSpan::Linewise { first: 0, last: 1 }
        );
    }

    #[test]
    fn de_crosses_to_word_end() {
        let lines = lines_of("foo\nbar");
        assert_eq!(cw_range(&lines, 0, 2, 1), charwise((0, 2), (1, 2)));
    }

    #[test]
    fn ce_crosses_to_word_end() {
        let lines = lines_of("foo\nbar");
        assert_eq!(ce_range(&lines, 0, 2, 1), charwise((0, 2), (1, 2)));
    }

    #[test]
    fn ce_matches_de_within_line() {
        let lines = lines_of("foo bar");
        assert_eq!(cw_range(&lines, 0, 4, 1), charwise((0, 4), (0, 6)));
        assert_eq!(ce_range(&lines, 0, 4, 1), charwise((0, 4), (0, 6)));
    }

    #[test]
    fn ce_skips_linewise_upgrade() {
        let lines = lines_of("\nbar");
        assert_eq!(ce_range(&lines, 0, 0, 1), charwise((0, 0), (1, 2)));
    }

    #[test]
    fn db_line_start_removes_prev_line() {
        let lines = lines_of("foo\nbar");
        assert_eq!(
            db_range(&lines, 1, 0, 1),
            WordSpan::Linewise { first: 0, last: 0 }
        );
    }

    #[test]
    fn db_across_lines_removes_middle() {
        let lines = lines_of("ab\ncd\nef");
        assert_eq!(
            db_range(&lines, 2, 0, 1),
            WordSpan::Linewise { first: 1, last: 1 }
        );
    }

    #[test]
    fn db_across_empty_takes_both_lines() {
        let lines = lines_of("foo\n  \nbar");
        assert_eq!(
            db_range(&lines, 2, 0, 1),
            WordSpan::Linewise { first: 0, last: 1 }
        );
    }

    #[test]
    fn db_same_line_deletes_back() {
        let lines = lines_of("foo bar");
        assert_eq!(db_range(&lines, 0, 4, 1), charwise((0, 0), (0, 3)));
    }

    #[test]
    fn db_at_start_is_empty() {
        let lines = lines_of("foo");
        assert_eq!(db_range(&lines, 0, 0, 1), WordSpan::Empty { at: (0, 0) });
    }

    #[test]
    fn de_last_empty_line_is_empty() {
        let lines = lines_of("a\n");
        assert_eq!(cw_range(&lines, 1, 0, 1), WordSpan::Empty { at: (1, 0) });
    }
}
