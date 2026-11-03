use crate::registry::SyntaxHighlight;
use crate::rows::{self, RowWindow};
use crate::span::SyntaxSpan;

pub(crate) struct Capture {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) highlight: SyntaxHighlight,
    pub(crate) layer: usize,
}

pub(crate) fn paint_captures(
    text: &str,
    rows: &[RowWindow],
    captures: &[Capture],
) -> Vec<SyntaxSpan> {
    let mut cells: Vec<Vec<Option<SyntaxHighlight>>> = rows
        .iter()
        .map(|row| vec![None; rows::row_char_len(text, row)])
        .collect();
    for capture in captures {
        paint_capture(text, rows, &mut cells, capture);
    }
    merge_cells(rows, cells)
}

fn paint_capture(
    text: &str,
    rows: &[RowWindow],
    cells: &mut [Vec<Option<SyntaxHighlight>>],
    capture: &Capture,
) {
    let first = rows.partition_point(|row| row.end <= capture.start);
    for (position, row) in rows.iter().enumerate().skip(first) {
        if row.start >= capture.end {
            break;
        }
        let start = capture.start.max(row.start);
        let end = capture.end.min(row.text_end);
        if start >= end {
            continue;
        }
        let from = rows::char_column(text, row.start, start);
        let to = rows::char_column(text, row.start, end);
        let line = &mut cells[position];
        let from = from.min(line.len());
        let to = to.min(line.len());
        for cell in &mut line[from..to] {
            *cell = Some(capture.highlight);
        }
    }
}

fn merge_cells(rows: &[RowWindow], cells: Vec<Vec<Option<SyntaxHighlight>>>) -> Vec<SyntaxSpan> {
    let mut spans = Vec::new();
    for (row, cells) in rows.iter().zip(cells) {
        let mut start = 0usize;
        let mut current = None;
        for (column, cell) in cells.iter().enumerate() {
            if *cell == current {
                continue;
            }
            if let Some(highlight) = current {
                spans.push(SyntaxSpan {
                    row: row.row,
                    start,
                    end: column,
                    highlight,
                });
            }
            current = *cell;
            start = column;
        }
        if let Some(highlight) = current {
            spans.push(SyntaxSpan {
                row: row.row,
                start,
                end: cells.len(),
                highlight,
            });
        }
    }
    spans
}
