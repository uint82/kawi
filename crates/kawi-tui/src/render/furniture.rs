use anyhow::{Context, Result};
use std::io::Write;

use crate::frame::PaneFrame;
use crate::paint;
use crate::surface::Surface;
use crate::theme::Theme;

pub(super) fn draw_rulers(
    out: &mut Surface,
    pane: &PaneFrame<'_>,
    theme: &Theme,
    screen_row: usize,
    gutter_width: usize,
    visible: usize,
) -> Result<()> {
    let width = pane.rect.width.saturating_sub(gutter_width);
    let leftcol = pane.viewport.leftcol();
    for ruler in pane.rulers {
        if *ruler < leftcol || *ruler >= leftcol.saturating_add(width) || visible > *ruler {
            continue;
        }
        let column = pane
            .rect
            .x
            .saturating_add(gutter_width)
            .saturating_add(ruler.saturating_sub(leftcol));
        out.move_to(column, pane.rect.y.saturating_add(screen_row));
        out.set_foreground(theme.non_text);
        write!(out, "|").context("write soft ruler")?;
        paint::restore_base(out, theme);
    }
    Ok(())
}

pub(super) fn erase_row_tail(out: &mut Surface) -> Result<()> {
    out.clear_tail();
    Ok(())
}

pub(super) fn draw_filler_rows(
    out: &mut Surface,
    pane: &PaneFrame<'_>,
    theme: &Theme,
) -> Result<()> {
    let height = pane.rect.height.saturating_sub(1);
    for screen_row in pane.lines.len()..height {
        out.move_to(pane.rect.x, pane.rect.y.saturating_add(screen_row));
        out.set_foreground(theme.non_text);
        if pane.signs.is_some() {
            write!(out, " ").context("write empty git sign")?;
        }
        write!(out, "~").context("write filler")?;
        paint::restore_base(out, theme);
        erase_row_tail(out)?;
    }
    Ok(())
}

pub(super) fn gutter_width(pane: &PaneFrame<'_>) -> usize {
    let numbers = pane.gutter.first().map_or(0, |line| line.chars().count());
    numbers
        .saturating_add(usize::from(pane.signs.is_some()))
        .saturating_add(pane.left_width)
}

pub(super) fn saturate_u16(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

pub(super) fn char_len(text: &str) -> usize {
    text.chars().count()
}

pub(super) fn truncate_to_width(text: &str, width: usize) -> String {
    text.chars().take(width).collect()
}

pub(super) fn write_padded(out: &mut Surface, text: &str, width: usize) -> Result<()> {
    let text = truncate_to_width(text, width);
    let padding = width.saturating_sub(text.chars().count());
    write!(out, "{text}{}", " ".repeat(padding)).context("write padded row")?;
    Ok(())
}
