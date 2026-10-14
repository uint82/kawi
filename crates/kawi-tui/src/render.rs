use anyhow::{Context, Result};
use crossterm::{
    cursor::{MoveTo, SetCursorStyle},
    execute,
    style::{Attribute, SetAttribute},
    terminal::{Clear, ClearType},
};
use std::io::{Stdout, Write};

use crate::frame::Frame;
use crate::statusbar::{draw_status, StatusInfo};
use crate::viewport::Viewport;

pub fn draw(out: &mut Stdout, frame: &Frame) -> Result<()> {
    clear_all(out)?;
    draw_visible_lines(out, frame.lines, frame.viewport)?;
    draw_highlight(out, frame)?;
    draw_status_row(out, frame.status, frame.viewport)?;
    draw_message_row(out, frame)?;
    apply_cursor_style(out, frame.insert_mode)?;
    place_cursor_relative(out, frame.col, frame.row, frame.viewport)?;
    flush(out)?;
    Ok(())
}

fn clear_all(out: &mut Stdout) -> Result<()> {
    execute!(out, Clear(ClearType::All)).context("clear screen")?;
    Ok(())
}

fn draw_visible_lines(out: &mut Stdout, lines: &[String], viewport: &Viewport) -> Result<()> {
    for (screen_row, line) in visible_slice(lines, viewport).iter().enumerate() {
        draw_one_line(out, screen_row, line, viewport.width())?;
    }
    Ok(())
}

fn visible_slice<'a>(lines: &'a [String], viewport: &Viewport) -> &'a [String] {
    let end = viewport.visible_end().min(lines.len());
    let start = viewport.top().min(end);
    &lines[start..end]
}

fn draw_one_line(out: &mut Stdout, screen_row: usize, text: &str, width: usize) -> Result<()> {
    execute!(out, MoveTo(0, saturate_u16(screen_row))).context("move to line start")?;
    write!(out, "{}", truncate_to_width(text, width)).context("write line")?;
    Ok(())
}

fn truncate_to_width(text: &str, width: usize) -> String {
    text.chars().take(width).collect()
}

pub struct HighlightSpan {
    pub screen_row: usize,
    pub from: usize,
    pub text: String,
}

pub fn highlight_spans(
    lines: &[String],
    selection: Option<((usize, usize), (usize, usize))>,
    viewport: &Viewport,
) -> Vec<HighlightSpan> {
    let Some(((start_row, start_col), (end_row, end_col))) = selection else {
        return Vec::new();
    };
    if (start_row, start_col) == (end_row, end_col) {
        return Vec::new();
    }
    let mut spans = Vec::new();
    for row in start_row..=end_row {
        collect_span(
            &mut spans,
            lines,
            viewport,
            row,
            ((start_row, start_col), (end_row, end_col)),
        );
    }
    spans
}

fn collect_span(
    spans: &mut Vec<HighlightSpan>,
    lines: &[String],
    viewport: &Viewport,
    row: usize,
    range: ((usize, usize), (usize, usize)),
) {
    if let Some(span) = selected_segment(lines, viewport, row, range) {
        spans.push(span);
    }
}

fn selected_segment(
    lines: &[String],
    viewport: &Viewport,
    row: usize,
    range: ((usize, usize), (usize, usize)),
) -> Option<HighlightSpan> {
    let ((start_row, start_col), (end_row, end_col)) = range;
    if !is_row_visible(viewport, row) {
        return None;
    }
    let text = lines.get(row)?;
    let width = viewport.width();
    let from = segment_start(row, start_row, start_col).min(width);
    let to = segment_end(row, text, end_row, end_col).min(width);
    if from >= to {
        return None;
    }
    let screen_row = row.saturating_sub(viewport.top());
    Some(HighlightSpan {
        screen_row,
        from,
        text: slice_cols(text, from, to),
    })
}

fn draw_highlight(out: &mut Stdout, frame: &Frame) -> Result<()> {
    for span in highlight_spans(frame.lines, frame.selection, frame.viewport) {
        paint_span(out, &span)?;
    }
    Ok(())
}

fn paint_span(out: &mut Stdout, span: &HighlightSpan) -> Result<()> {
    move_to_segment(out, span.from, span.screen_row)?;
    enable_reverse(out)?;
    write_segment(out, &span.text)?;
    disable_reverse(out)?;
    Ok(())
}

fn is_row_visible(viewport: &Viewport, row: usize) -> bool {
    row >= viewport.top() && row < viewport.visible_end()
}

fn segment_start(row: usize, start_row: usize, start_col: usize) -> usize {
    if row == start_row {
        start_col
    } else {
        0
    }
}

fn segment_end(row: usize, text: &str, end_row: usize, end_col: usize) -> usize {
    if row == end_row {
        end_col.saturating_add(1)
    } else {
        char_len(text)
    }
}

fn slice_cols(text: &str, from: usize, to: usize) -> String {
    text.chars()
        .skip(from)
        .take(to.saturating_sub(from))
        .collect()
}

fn move_to_segment(out: &mut Stdout, col: usize, screen_row: usize) -> Result<()> {
    execute!(out, MoveTo(saturate_u16(col), saturate_u16(screen_row)))
        .context("move to segment")?;
    Ok(())
}

fn enable_reverse(out: &mut Stdout) -> Result<()> {
    execute!(out, SetAttribute(Attribute::Reverse)).context("enable reverse")?;
    Ok(())
}

fn write_segment(out: &mut Stdout, text: &str) -> Result<()> {
    write!(out, "{text}").context("write segment")?;
    Ok(())
}

fn disable_reverse(out: &mut Stdout) -> Result<()> {
    execute!(out, SetAttribute(Attribute::Reset)).context("disable reverse")?;
    Ok(())
}

fn draw_status_row(out: &mut Stdout, status: &StatusInfo, viewport: &Viewport) -> Result<()> {
    draw_status(out, status, viewport.width(), viewport.height())
}

fn draw_message_row(out: &mut Stdout, frame: &Frame) -> Result<()> {
    let Some(message) = frame.message else {
        return Ok(());
    };
    draw_message_text(out, message, frame.viewport)
}

fn draw_message_text(out: &mut Stdout, message: &str, viewport: &Viewport) -> Result<()> {
    execute!(
        out,
        MoveTo(0, saturate_u16(viewport.height().saturating_add(1)))
    )
    .context("move to message row")?;
    write!(out, "{}", truncate_to_width(message, viewport.width())).context("write message")?;
    Ok(())
}

fn apply_cursor_style(out: &mut Stdout, insert_mode: bool) -> Result<()> {
    execute!(out, cursor_style(insert_mode)).context("set cursor style")?;
    Ok(())
}

fn cursor_style(insert_mode: bool) -> SetCursorStyle {
    if insert_mode {
        SetCursorStyle::SteadyBar
    } else {
        SetCursorStyle::SteadyBlock
    }
}

fn place_cursor_relative(
    out: &mut Stdout,
    col: usize,
    row: usize,
    viewport: &Viewport,
) -> Result<()> {
    let (x, y) = screen_coords(col, row, viewport);
    execute!(out, MoveTo(x, y)).context("place cursor")?;
    Ok(())
}

fn screen_coords(col: usize, row: usize, viewport: &Viewport) -> (u16, u16) {
    let x = saturate_u16(col.min(viewport.width().saturating_sub(1)));
    let y = saturate_u16(viewport.screen_row(row));
    (x, y)
}

fn flush(out: &mut Stdout) -> Result<()> {
    out.flush().context("flush stdout")?;
    Ok(())
}

fn char_len(text: &str) -> usize {
    text.chars().count()
}

fn saturate_u16(v: usize) -> u16 {
    u16::try_from(v).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewport() -> Viewport {
        Viewport::new(10, 80)
    }

    fn lines_of(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
    }

    #[test]
    fn no_selection_has_no_spans() {
        let lines = lines_of("abcdef");
        assert!(highlight_spans(&lines, None, &viewport()).is_empty());
    }

    #[test]
    fn point_selection_has_no_spans() {
        let lines = lines_of("abcdef");
        let spans = highlight_spans(&lines, Some(((0, 2), (0, 2))), &viewport());
        assert!(spans.is_empty());
    }

    #[test]
    fn range_selection_spans_chars() {
        let lines = lines_of("abcdef");
        let spans = highlight_spans(&lines, Some(((0, 1), (0, 3))), &viewport());
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].screen_row, 0);
        assert_eq!(spans[0].from, 1);
        assert_eq!(spans[0].text, "bcd");
    }

    #[test]
    fn multiline_selection_spans_rows() {
        let lines = lines_of("ab\ncd\nef");
        let spans = highlight_spans(&lines, Some(((0, 1), (2, 1))), &viewport());
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].text, "b");
        assert_eq!(spans[1].text, "cd");
        assert_eq!(spans[2].text, "ef");
    }
}
