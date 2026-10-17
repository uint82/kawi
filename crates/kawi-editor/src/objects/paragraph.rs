use kawi_buffer::Buffer;

use crate::paragraphs::is_paragraph_start;
use crate::words::WordSpan;

pub fn paragraph_range(
    buffer: &Buffer,
    row: usize,
    include: bool,
    count: usize,
) -> Option<WordSpan> {
    let (first, last) = paragraph_span(buffer, row, include, count)?;
    Some(WordSpan::Linewise { first, last })
}

fn paragraph_span(
    buffer: &Buffer,
    row: usize,
    include: bool,
    count: usize,
) -> Option<(usize, usize)> {
    let line_count = buffer.line_count() as isize;
    let mut start = row as isize;
    let white_in_front = line_blank(buffer, start);
    while start > 0 {
        if white_in_front {
            if !line_blank(buffer, start - 1) {
                break;
            }
        } else if line_blank(buffer, start - 1) || is_paragraph_start(buffer, start as usize) {
            break;
        }
        start -= 1;
    }
    let mut end = start;
    while end < line_count && line_blank(buffer, end) {
        end += 1;
    }
    end -= 1;
    let mut remaining = count as isize;
    if !include && white_in_front {
        remaining -= 1;
    }
    let mut do_white = false;
    while remaining > 0 {
        remaining -= 1;
        if end == line_count - 1 {
            return None;
        }
        if !include {
            do_white = line_blank(buffer, end + 1);
        }
        if include || !do_white {
            end += 1;
            while end < line_count - 1
                && !line_blank(buffer, end + 1)
                && !is_paragraph_start(buffer, (end + 1) as usize)
            {
                end += 1;
            }
        }
        if remaining == 0 && white_in_front && include {
            break;
        }
        if include || do_white {
            while end < line_count - 1 && line_blank(buffer, end + 1) {
                end += 1;
            }
        }
    }
    if !white_in_front && !line_blank(buffer, end) && include {
        while start > 0 && line_blank(buffer, start - 1) {
            start -= 1;
        }
    }
    Some((start as usize, end as usize))
}

pub fn paragraph_step(buffer: &Buffer, row: usize, forward: bool, include: bool) -> usize {
    let last = buffer.line_count().saturating_sub(1) as isize;
    let edge = if forward { last } else { 0 };
    let mut line = row as isize;
    if line == edge {
        return row;
    }
    let dir = if forward { 1 } else { -1 };
    let mut prev_white: Option<bool> = None;
    for _ in 0..2 {
        line += dir;
        let white = line_blank(buffer, line);
        if prev_white == Some(white) {
            line -= dir;
            break;
        }
        loop {
            if line == edge {
                break;
            }
            if white != line_blank(buffer, line + dir) {
                break;
            }
            if !white && is_paragraph_start(buffer, (line + if forward { 1 } else { 0 }) as usize) {
                break;
            }
            line += dir;
        }
        if !include {
            break;
        }
        if line == edge {
            break;
        }
        prev_white = Some(white);
    }
    line as usize
}

fn line_blank(buffer: &Buffer, row: isize) -> bool {
    if row < 0 {
        return false;
    }
    match buffer.line_cow(row as usize) {
        None => false,
        Some(line) => line.chars().all(|ch| ch.is_whitespace()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_buffer::Buffer;

    fn lines_of(text: &str) -> Buffer {
        Buffer::from_string(text)
    }

    fn para(buffer: &Buffer, row: usize, inner: bool, count: usize) -> Option<WordSpan> {
        paragraph_range(buffer, row, !inner, count)
    }

    fn ln(first: usize, last: usize) -> WordSpan {
        WordSpan::Linewise { first, last }
    }

    #[test]
    fn inner_paragraph_at_macro_line() {
        let lines = lines_of("intro line\n.PP\npara two\n.IP tag\npara three\n.SH NAME\ntail");
        assert_eq!(para(&lines, 0, true, 1), Some(ln(0, 0)));
        assert_eq!(para(&lines, 1, true, 1), Some(ln(1, 2)));
        assert_eq!(para(&lines, 3, true, 1), Some(ln(3, 4)));
        assert_eq!(para(&lines, 5, true, 1), Some(ln(5, 6)));
    }

    #[test]
    fn around_paragraph_at_macro_line() {
        let lines = lines_of("intro line\n.PP\npara two\n.IP tag\npara three\n.SH NAME\ntail");
        assert_eq!(para(&lines, 0, false, 1), Some(ln(0, 0)));
        assert_eq!(para(&lines, 1, false, 1), Some(ln(1, 2)));
    }

    #[test]
    fn paragraphs_next_to_macro_line() {
        let lines = lines_of("a\n\nb\n.PP\nc\n\nd");
        assert_eq!(para(&lines, 1, false, 1), Some(ln(1, 2)));
        assert_eq!(para(&lines, 4, true, 1), Some(ln(3, 4)));
        assert_eq!(para(&lines, 4, false, 1), Some(ln(3, 5)));
        assert_eq!(para(&lines, 5, true, 1), Some(ln(5, 5)));
        assert_eq!(para(&lines, 5, false, 1), Some(ln(5, 6)));
    }

    #[test]
    fn inner_first_paragraph() {
        let lines = lines_of("a\n\nb\n\nc");
        assert_eq!(para(&lines, 0, true, 1), Some(ln(0, 0)));
    }

    #[test]
    fn around_first_paragraph() {
        let lines = lines_of("a\n\nb\n\nc");
        assert_eq!(para(&lines, 0, false, 1), Some(ln(0, 1)));
    }

    #[test]
    fn inner_middle_paragraph() {
        let lines = lines_of("a\n\nb\n\nc");
        assert_eq!(para(&lines, 2, true, 1), Some(ln(2, 2)));
    }

    #[test]
    fn around_middle_paragraph() {
        let lines = lines_of("a\n\nb\n\nc");
        assert_eq!(para(&lines, 2, false, 1), Some(ln(2, 3)));
    }

    #[test]
    fn inner_blank_deletes_blank() {
        let lines = lines_of("a\n\nb\n\nc");
        assert_eq!(para(&lines, 1, true, 1), Some(ln(1, 1)));
    }

    #[test]
    fn around_blank_takes_next() {
        let lines = lines_of("a\n\nb\n\nc");
        assert_eq!(para(&lines, 1, false, 1), Some(ln(1, 2)));
    }

    #[test]
    fn inner_first_blank() {
        let lines = lines_of("\na");
        assert_eq!(para(&lines, 0, true, 1), Some(ln(0, 0)));
    }

    #[test]
    fn inner_last_blank() {
        let lines = lines_of("a\n\n");
        assert_eq!(para(&lines, 1, true, 1), Some(ln(1, 1)));
    }

    #[test]
    fn inner_whitespace_counts_blank() {
        let lines = lines_of("a\n   \nb");
        assert_eq!(para(&lines, 0, true, 1), Some(ln(0, 0)));
    }

    #[test]
    fn inner_solo_line() {
        let lines = lines_of("a");
        assert_eq!(para(&lines, 0, true, 1), Some(ln(0, 0)));
    }

    #[test]
    fn around_solo_line() {
        let lines = lines_of("a");
        assert_eq!(para(&lines, 0, false, 1), Some(ln(0, 0)));
    }

    #[test]
    fn inner_count_extends() {
        let lines = lines_of("a\n\nb\n\nc");
        assert_eq!(para(&lines, 2, true, 2), Some(ln(2, 3)));
    }

    #[test]
    fn around_count_extends() {
        let lines = lines_of("a\n\nb\n\nc");
        assert_eq!(para(&lines, 2, false, 2), Some(ln(1, 4)));
    }

    #[test]
    fn around_count_from_start() {
        let lines = lines_of("a\n\nb\n\nc");
        assert_eq!(para(&lines, 0, false, 2), Some(ln(0, 3)));
    }
}
