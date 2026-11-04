use anyhow::{Context, Result};
use std::io::Write;

use crate::paint;
use crate::surface::Surface;
use crate::theme::{MatchColors, Theme};

use super::layout::StatusLine;
use super::segment::SegmentStyle;

pub fn draw_line(
    out: &mut Surface,
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
        paint::apply_match(out, &colors);
        out.move_to(x.saturating_add(span.x), y);
        for _ in 0..span.padding {
            write!(out, " ").context("write status segment padding")?;
        }
        write!(out, "{}", span.text).context("write status segment")?;
        for _ in 0..span.padding {
            write!(out, " ").context("write status segment padding")?;
        }
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
    paint::restore_base(out, theme);
    Ok(())
}

fn write_cells(
    out: &mut Surface,
    x: usize,
    y: usize,
    count: usize,
    colors: &MatchColors,
) -> Result<()> {
    if count == 0 {
        return Ok(());
    }
    paint::apply_match(out, colors);
    out.move_to(x, y);
    write!(out, "{:width$}", "", width = count).context("write status padding")?;
    Ok(())
}
