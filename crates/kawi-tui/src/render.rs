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

fn draw_highlight(out: &mut Stdout, frame: &Frame) -> Result<()> {
    let Some(((start_row, start_col), (end_row, end_col))) = frame.selection else {
        return Ok(());
    };
    for row in start_row..=end_row {
        paint_selected_segment(out, frame, row, start_row, start_col, end_row, end_col)?;
    }
    Ok(())
}

fn paint_selected_segment(
    out: &mut Stdout,
    frame: &Frame,
    row: usize,
    start_row: usize,
    start_col: usize,
    end_row: usize,
    end_col: usize,
) -> Result<()> {
    let Some((screen_row, text, from)) =
        selected_segment(frame, row, start_row, start_col, end_row, end_col)
    else {
        return Ok(());
    };
    move_to_segment(out, from, screen_row)?;
    enable_reverse(out)?;
    write_segment(out, &text)?;
    disable_reverse(out)?;
    Ok(())
}

fn selected_segment(
    frame: &Frame,
    row: usize,
    start_row: usize,
    start_col: usize,
    end_row: usize,
    end_col: usize,
) -> Option<(usize, String, usize)> {
    if !is_row_visible(frame.viewport, row) {
        return None;
    }
    let text = frame.lines.get(row)?;
    let width = frame.viewport.width();
    let from = segment_start(row, start_row, start_col).min(width);
    let to = segment_end(row, text, end_row, end_col).min(width);
    if from >= to {
        return None;
    }
    let screen_row = row.saturating_sub(frame.viewport.top());
    Some((screen_row, slice_cols(text, from, to), from))
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

fn apply_cursor_style(out: &mut Stdout, insert_mode: bool) -> Result<()> {
    execute!(out, cursor_style(insert_mode)).context("set cursor style")?;
    Ok(())
}

fn cursor_style(insert_mode: bool) -> SetCursorStyle {
    if insert_mode {
        crossterm::cursor::SetCursorStyle::SteadyBar
    } else {
        crossterm::cursor::SetCursorStyle::SteadyBlock
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
