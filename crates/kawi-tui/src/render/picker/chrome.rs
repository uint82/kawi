use anyhow::{Context, Result};
use std::io::Write;

use crate::frame::{Frame, Rect};
use crate::paint;
use crate::surface::Surface;
use crate::text::{display_width, truncate_to_display_width};

pub(super) fn draw_box(out: &mut Surface, frame: &Frame, rect: Rect, title: &str) -> Result<()> {
    if rect.width < 2 || rect.height < 2 {
        return Ok(());
    }
    paint::apply_colors(out, frame.theme.non_text, frame.theme.float_background);
    let inner = rect.width.saturating_sub(2);
    let label = format!(" {title} ");
    let label = truncate_to_display_width(&label, inner);
    let fill = inner.saturating_sub(display_width(label));
    let top = format!("╭{label}{}╮", "─".repeat(fill));
    let bottom = format!("╰{}╯", "─".repeat(inner));
    out.move_to(rect.x, rect.y);
    write!(out, "{top}").context("write picker top border")?;
    out.move_to(rect.x, rect.bottom().saturating_sub(1));
    write!(out, "{bottom}").context("write picker bottom border")?;
    draw_side_borders(out, rect)
}

fn draw_side_borders(out: &mut Surface, rect: Rect) -> Result<()> {
    for row in rect.y.saturating_add(1)..rect.bottom().saturating_sub(1) {
        out.move_to(rect.x, row);
        write!(out, "│").context("write picker left border")?;
        out.move_to(rect.right().saturating_sub(1), row);
        write!(out, "│").context("write picker right border")?;
    }
    Ok(())
}

pub(super) fn draw_rule(out: &mut Surface, frame: &Frame, rect: Rect) -> Result<()> {
    if rect.width == 0 || rect.height == 0 {
        return Ok(());
    }
    out.move_to(rect.x, rect.y);
    out.set_foreground(frame.theme.non_text);
    write!(out, "{}", "─".repeat(rect.width)).context("write picker rule")?;
    Ok(())
}

pub(super) fn draw_divider(out: &mut Surface, frame: &Frame, divider: Option<Rect>) -> Result<()> {
    let Some(rect) = divider else {
        return Ok(());
    };
    if rect.width == 0 || rect.height == 0 {
        return Ok(());
    }
    out.set_foreground(frame.theme.non_text);
    let glyph = if rect.width == 1 { "│" } else { "─" };
    let line = glyph.repeat(rect.width);
    for row in rect.y..rect.bottom() {
        out.move_to(rect.x, row);
        write!(out, "{line}").context("write picker divider")?;
    }
    Ok(())
}
