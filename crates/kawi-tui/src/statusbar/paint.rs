use anyhow::{Context, Result};
use crossterm::{cursor::MoveTo, execute};
use std::io::{Stdout, Write};

use crate::paint;
use crate::theme::{MatchColors, Theme};

use super::layout::StatusLine;
use super::segment::SegmentStyle;

pub fn draw_line(
    out: &mut Stdout,
    line: &StatusLine,
    x: usize,
    y: usize,
    active: bool,
    theme: &Theme,
) -> Result<()> {
    let default = theme.statusline_colors(SegmentStyle::Default, active);
    let mut cursor = 0;
    for span in &line.spans {
        if span.x > cursor {
            write_cells(out, x.saturating_add(cursor), y, span.x - cursor, &default)?;
        }
        let colors = theme.statusline_colors(span.style, active);
        paint::apply_match(out, &colors)?;
        execute!(
            out,
            MoveTo(saturate_u16(x.saturating_add(span.x)), saturate_u16(y))
        )
        .context("move to status segment")?;
        write!(out, "{}", span.text).context("write status segment")?;
        cursor = span.x.saturating_add(span.width);
    }
    if cursor < line.width {
        write_cells(
            out,
            x.saturating_add(cursor),
            y,
            line.width - cursor,
            &default,
        )?;
    }
    paint::restore_base(out, theme)?;
    Ok(())
}

fn write_cells(
    out: &mut Stdout,
    x: usize,
    y: usize,
    count: usize,
    colors: &MatchColors,
) -> Result<()> {
    if count == 0 {
        return Ok(());
    }
    paint::apply_match(out, colors)?;
    execute!(out, MoveTo(saturate_u16(x), saturate_u16(y))).context("move to status padding")?;
    write!(out, "{:width$}", "", width = count).context("write status padding")?;
    Ok(())
}

fn saturate_u16(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}
