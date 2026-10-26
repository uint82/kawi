use anyhow::{Context, Result};
use crossterm::{cursor::MoveTo, execute, style::SetForegroundColor};
use std::io::{Stdout, Write};

use super::super::saturate_u16;
use crate::frame::{Frame, Rect};
use crate::paint;
use crate::text::{display_width, truncate_to_display_width};

pub(super) fn draw_box(out: &mut Stdout, frame: &Frame, rect: Rect, title: &str) -> Result<()> {
    if rect.width < 2 || rect.height < 2 {
        return Ok(());
    }
    paint::apply_colors(
        out,
        frame.theme.non_text,
        frame.theme.float_background,
        "apply picker border colors",
    )?;
    let inner = rect.width.saturating_sub(2);
    let label = format!(" {title} ");
    let label = truncate_to_display_width(&label, inner);
    let fill = inner.saturating_sub(display_width(label));
    let top = format!("╭{label}{}╮", "─".repeat(fill));
    let bottom = format!("╰{}╯", "─".repeat(inner));
    execute!(out, MoveTo(saturate_u16(rect.x), saturate_u16(rect.y)))
        .context("move to picker top border")?;
    write!(out, "{top}").context("write picker top border")?;
    execute!(
        out,
        MoveTo(
            saturate_u16(rect.x),
            saturate_u16(rect.bottom().saturating_sub(1))
        )
    )
    .context("move to picker bottom border")?;
    write!(out, "{bottom}").context("write picker bottom border")?;
    draw_side_borders(out, rect)
}

fn draw_side_borders(out: &mut Stdout, rect: Rect) -> Result<()> {
    for row in rect.y.saturating_add(1)..rect.bottom().saturating_sub(1) {
        execute!(out, MoveTo(saturate_u16(rect.x), saturate_u16(row)))
            .context("move to picker left border")?;
        write!(out, "│").context("write picker left border")?;
        execute!(
            out,
            MoveTo(
                saturate_u16(rect.right().saturating_sub(1)),
                saturate_u16(row)
            )
        )
        .context("move to picker right border")?;
        write!(out, "│").context("write picker right border")?;
    }
    Ok(())
}

pub(super) fn draw_rule(out: &mut Stdout, frame: &Frame, rect: Rect) -> Result<()> {
    if rect.width == 0 || rect.height == 0 {
        return Ok(());
    }
    execute!(
        out,
        MoveTo(saturate_u16(rect.x), saturate_u16(rect.y)),
        SetForegroundColor(frame.theme.non_text)
    )
    .context("prepare picker rule")?;
    write!(out, "{}", "─".repeat(rect.width)).context("write picker rule")?;
    Ok(())
}

pub(super) fn draw_divider(out: &mut Stdout, frame: &Frame, divider: Option<Rect>) -> Result<()> {
    let Some(rect) = divider else {
        return Ok(());
    };
    if rect.width == 0 || rect.height == 0 {
        return Ok(());
    }
    execute!(out, SetForegroundColor(frame.theme.non_text))
        .context("apply picker divider color")?;
    let glyph = if rect.width == 1 { "│" } else { "─" };
    let line = glyph.repeat(rect.width);
    for row in rect.y..rect.bottom() {
        execute!(out, MoveTo(saturate_u16(rect.x), saturate_u16(row)))
            .context("move to picker divider")?;
        write!(out, "{line}").context("write picker divider")?;
    }
    Ok(())
}
