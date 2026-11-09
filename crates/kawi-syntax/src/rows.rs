use std::ops::Range;

use crate::diff::LineIndex;
use crate::limits::MAX_LINE_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RowWindow {
    pub row: usize,
    pub start: usize,
    pub text_end: usize,
    pub end: usize,
}

pub(crate) fn visible_rows(
    text: &str,
    index: &LineIndex,
    top: usize,
    end: usize,
) -> Vec<RowWindow> {
    let end = end.min(index.line_count());
    (top..end)
        .filter_map(|row| row_window(text, index, row))
        .collect()
}

fn row_window(text: &str, index: &LineIndex, row: usize) -> Option<RowWindow> {
    let start = index.line_start(row);
    let end = next_line_start(text, index, row);
    let text_end = trim_ending(text, start, end);
    if text_end.saturating_sub(start) > MAX_LINE_BYTES {
        return None;
    }
    Some(RowWindow {
        row,
        start,
        text_end,
        end,
    })
}

fn next_line_start(text: &str, index: &LineIndex, row: usize) -> usize {
    if row.saturating_add(1) < index.line_count() {
        index.line_start(row.saturating_add(1))
    } else {
        text.len()
    }
}

pub(crate) fn line_text_end(text: &str, index: &LineIndex, row: usize) -> usize {
    let start = index.line_start(row);
    let end = next_line_start(text, index, row);
    trim_ending(text, start, end)
}

fn trim_ending(text: &str, start: usize, end: usize) -> usize {
    let mut end = end;
    while end > start {
        match text.as_bytes()[end - 1] {
            b'\n' | b'\r' => end -= 1,
            _ => break,
        }
    }
    end
}

pub(crate) fn query_blocks(rows: &[RowWindow]) -> Vec<Range<usize>> {
    let mut blocks = Vec::new();
    let mut current: Option<Range<usize>> = None;
    for row in rows {
        current = match current {
            Some(range) if range.end == row.start => Some(range.start..row.end),
            Some(range) => {
                blocks.push(range);
                Some(row.start..row.end)
            }
            None => Some(row.start..row.end),
        };
    }
    blocks.extend(current);
    blocks
}

pub(crate) fn row_char_len(text: &str, row: &RowWindow) -> usize {
    text[row.start..row.text_end].chars().count()
}

pub(crate) fn char_column(text: &str, line_start: usize, byte: usize) -> usize {
    let mut byte = byte.clamp(line_start, text.len());
    while byte > line_start && !text.is_char_boundary(byte) {
        byte -= 1;
    }
    text[line_start..byte].chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows_of(text: &str, top: usize, end: usize) -> Vec<RowWindow> {
        visible_rows(text, &LineIndex::new(text), top, end)
    }

    #[test]
    fn rows_exclude_line_endings() {
        let rows = rows_of("ab\ncd\n", 0, 2);
        assert_eq!(
            rows,
            vec![
                RowWindow {
                    row: 0,
                    start: 0,
                    text_end: 2,
                    end: 3
                },
                RowWindow {
                    row: 1,
                    start: 3,
                    text_end: 5,
                    end: 6
                }
            ]
        );
    }

    #[test]
    fn rows_handle_missing_trailing_newline() {
        let rows = rows_of("ab\ncd", 0, 2);
        assert_eq!(rows[1].text_end, 5);
        assert_eq!(rows[1].end, 5);
    }

    #[test]
    fn rows_strip_carriage_returns() {
        let rows = rows_of("ab\r\ncd\r\n", 0, 2);
        assert_eq!(rows[0].text_end, 2);
        assert_eq!(rows[1].text_end, 6);
    }

    #[test]
    fn long_lines_are_not_highlighted() {
        let long = "x".repeat(MAX_LINE_BYTES + 1);
        let text = format!("{long}\nshort\n");
        let rows = rows_of(&text, 0, 2);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].row, 1);
    }

    #[test]
    fn blocks_split_around_skipped_rows() {
        let long = "x".repeat(MAX_LINE_BYTES + 1);
        let text = format!("a\n{long}\nb\nc\n");
        let rows = rows_of(&text, 0, 4);
        let blocks = query_blocks(&rows);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0], 0..2);
        assert_eq!(blocks[1], text.find("b\n").unwrap()..text.len());
    }

    #[test]
    fn blocks_join_consecutive_rows() {
        let rows = rows_of("ab\ncd\nef\n", 0, 3);
        assert_eq!(query_blocks(&rows), vec![0.."ab\ncd\nef\n".len()]);
    }

    #[test]
    fn character_columns_count_unicode() {
        let text = "éx";
        assert_eq!(char_column(text, 0, 0), 0);
        assert_eq!(char_column(text, 0, 2), 1);
        assert_eq!(char_column(text, 0, 3), 2);
    }

    #[test]
    fn row_char_len_ignores_line_endings() {
        let rows = rows_of("é\nx\n", 0, 2);
        assert_eq!(row_char_len("é\nx\n", &rows[0]), 1);
        assert_eq!(row_char_len("é\nx\n", &rows[1]), 1);
    }
}
