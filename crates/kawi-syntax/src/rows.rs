use std::ops::Range;

use ropey::Rope;

use crate::limits::MAX_LINE_BYTES;
use crate::text;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RowWindow {
    pub row: usize,
    pub start: usize,
    pub text_end: usize,
    pub end: usize,
}

pub(crate) fn visible_rows(text: &Rope, top: usize, end: usize) -> Vec<RowWindow> {
    let end = end.min(text::line_count(text));
    (top..end).filter_map(|row| row_window(text, row)).collect()
}

fn row_window(text: &Rope, row: usize) -> Option<RowWindow> {
    let start = text::line_start(text, row);
    let text_end = text::line_text_end(text, row);
    if text_end.saturating_sub(start) > MAX_LINE_BYTES {
        return None;
    }
    Some(RowWindow {
        row,
        start,
        text_end,
        end: text::line_end(text, row),
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn rope(text: &str) -> Rope {
        Rope::from_str(text)
    }

    fn rows_of(text: &str, top: usize, end: usize) -> Vec<RowWindow> {
        visible_rows(&rope(text), top, end)
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
}
