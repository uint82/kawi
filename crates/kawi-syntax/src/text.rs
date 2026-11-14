use std::ops::Range;

use ropey::Rope;
use tree_sitter::Point;

pub(crate) fn line_count(text: &Rope) -> usize {
    text.len_lines()
}

pub(crate) fn line_start(text: &Rope, row: usize) -> usize {
    if row < text.len_lines() {
        text.line_to_byte(row)
    } else {
        text.len_bytes()
    }
}

pub(crate) fn line_end(text: &Rope, row: usize) -> usize {
    line_start(text, row.saturating_add(1))
}

pub(crate) fn line_text_end(text: &Rope, row: usize) -> usize {
    trim_ending(text, line_start(text, row), line_end(text, row))
}

pub(crate) fn row_char_len(text: &Rope, row: usize) -> usize {
    let start = line_start(text, row);
    let end = line_text_end(text, row);
    char_index(text, end).saturating_sub(char_index(text, start))
}

pub(crate) fn char_column(text: &Rope, line_start: usize, byte: usize) -> usize {
    char_index(text, byte).saturating_sub(char_index(text, line_start))
}

pub(crate) fn char_index(text: &Rope, byte: usize) -> usize {
    text.byte_to_char(char_boundary(text, byte))
}

pub(crate) fn point(text: &Rope, byte: usize) -> Point {
    let byte = char_boundary(text, byte);
    let row = text.byte_to_line(byte);
    Point::new(row, byte.saturating_sub(text.line_to_byte(row)))
}

pub(crate) fn byte_range(text: &Rope, chars: Range<usize>) -> Range<usize> {
    let start = chars.start.min(text.len_chars());
    let end = chars.end.min(text.len_chars()).max(start);
    text.char_to_byte(start)..text.char_to_byte(end)
}

pub(crate) fn slice(text: &Rope, range: Range<usize>) -> Option<String> {
    text.get_byte_slice(range).map(|slice| slice.to_string())
}

fn trim_ending(text: &Rope, start: usize, end: usize) -> usize {
    let mut end = end;
    while end > start {
        match text.get_byte(end - 1) {
            Some(b'\n' | b'\r') => end -= 1,
            _ => break,
        }
    }
    end
}

fn char_boundary(text: &Rope, byte: usize) -> usize {
    let mut byte = byte.min(text.len_bytes());
    while byte > 0 && text.try_byte_to_char(byte).is_err() {
        byte -= 1;
    }
    byte
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rope(text: &str) -> Rope {
        Rope::from_str(text)
    }

    #[test]
    fn points_use_rows_and_byte_columns() {
        let text = rope("ab\ncd\n");
        assert_eq!(point(&text, 0), Point::new(0, 0));
        assert_eq!(point(&text, 3), Point::new(1, 0));
        assert_eq!(point(&text, 4), Point::new(1, 1));
        assert_eq!(point(&text, text.len_bytes()), Point::new(2, 0));
    }

    #[test]
    fn lines_match_rope_line_boundaries() {
        let text = rope("ab\ncd\n");
        assert_eq!(line_count(&text), 3);
        assert_eq!(line_start(&text, 0), 0);
        assert_eq!(line_start(&text, 1), 3);
        assert_eq!(line_start(&text, 2), 6);
        assert_eq!(line_end(&text, 0), 3);
        assert_eq!(line_end(&text, 2), 6);
    }

    #[test]
    fn line_text_end_strips_crlf() {
        let text = rope("ab\r\ncd");
        assert_eq!(line_text_end(&text, 0), 2);
        assert_eq!(line_text_end(&text, 1), 6);
    }

    #[test]
    fn char_columns_count_unicode() {
        let text = rope("éx");
        assert_eq!(char_column(&text, 0, 0), 0);
        assert_eq!(char_column(&text, 0, 2), 1);
        assert_eq!(char_column(&text, 0, 3), 2);
    }

    #[test]
    fn row_char_len_ignores_line_endings() {
        let text = rope("é\r\nx\n");
        assert_eq!(row_char_len(&text, 0), 1);
        assert_eq!(row_char_len(&text, 1), 1);
    }

    #[test]
    fn char_indices_snap_inside_unicode() {
        let text = rope("éx");
        assert_eq!(char_index(&text, 1), 0);
        assert_eq!(char_index(&text, 2), 1);
    }

    #[test]
    fn byte_ranges_convert_from_char_offsets() {
        let text = rope("a𝄞b");
        assert_eq!(byte_range(&text, 1..2), 1..5);
    }

    #[test]
    fn slices_return_owned_text() {
        let text = rope("alpha\nbeta");
        assert_eq!(slice(&text, 6..10).as_deref(), Some("beta"));
        assert_eq!(slice(&text, 99..100), None);
    }
}
