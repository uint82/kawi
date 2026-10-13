use anyhow::{Context, Result};
use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{Clear, ClearType},
};
use std::io::{Stdout, Write};

use crate::statusbar::{draw_status, StatusInfo};
use crate::viewport::Viewport;

pub fn draw(
    out: &mut Stdout,
    lines: &[String],
    col: usize,
    row: usize,
    viewport: &Viewport,
    status: &StatusInfo,
) -> Result<()> {
    clear_all(out)?;
    draw_visible_lines(out, lines, viewport)?;
    draw_status_row(out, status, viewport)?;
    place_cursor_relative(out, col, row, viewport)?;
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

fn draw_status_row(out: &mut Stdout, status: &StatusInfo, viewport: &Viewport) -> Result<()> {
    draw_status(out, status, viewport.width(), viewport.height())
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

fn saturate_u16(v: usize) -> u16 {
    u16::try_from(v).unwrap_or(u16::MAX)
}
