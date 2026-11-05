use std::ops::Range;

use crossterm::style::Color;
use kawi_lsp::DocumentColor;
use kawi_tui::frame::DecorationSpan;

use super::color::Rgba;

pub(super) fn spans(
    colors: &[DocumentColor],
    lines: &[String],
    rows: &[usize],
    background: Color,
) -> Vec<DecorationSpan> {
    let mut spans = Vec::new();
    for color in colors {
        for (line, row) in lines.iter().zip(rows.iter().copied()) {
            let Some(range) = visible_range(color, row, line) else {
                continue;
            };
            spans.push(DecorationSpan {
                row,
                start: range.start,
                end: range.end,
                style: Rgba::new(color.red, color.green, color.blue, color.alpha)
                    .to_style(background),
            });
        }
    }
    spans
}

fn visible_range(color: &DocumentColor, row: usize, line: &str) -> Option<Range<usize>> {
    let line_number = u32::try_from(row).ok()?;
    if line_number < color.start.line || line_number > color.end.line {
        return None;
    }
    let line_len = line.chars().count();
    let start = if line_number == color.start.line {
        color.start.column
    } else {
        0
    };
    let end = if line_number == color.end.line {
        color.end.column
    } else {
        u32::try_from(line_len).unwrap_or(u32::MAX)
    };
    let start = clamp_column(start, line_len);
    let end = clamp_column(end, line_len);
    (start < end).then_some(start..end)
}

fn clamp_column(column: u32, line_len: usize) -> usize {
    usize::try_from(column).unwrap_or(usize::MAX).min(line_len)
}
