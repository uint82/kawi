use anyhow::{Context, Result};
use std::io::Write;

use crate::frame::PaneFrame;
use crate::paint;
use crate::surface::Surface;
use crate::text::{display_width, truncate_to_display_width};
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

pub(super) fn write_spaces(out: &mut Surface, width: usize) -> Result<()> {
    if width == 0 {
        return Ok(());
    }
    write!(out, "{}", " ".repeat(width)).context("write spaces")?;
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

pub(super) fn write_padded(out: &mut Surface, text: &str, width: usize) -> Result<()> {
    let text = truncate_to_display_width(text, width);
    let padding = width.saturating_sub(display_width(text));
    write!(out, "{text}{}", " ".repeat(padding)).context("write padded row")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crossterm::style::Color;

    use super::*;

    #[test]
    fn pads_by_display_width() {
        let mut out = Surface::new(4, 1, Color::White, Color::Black);
        write_padded(&mut out, "\u{4e2d}", 4).expect("write padded row");
        assert_eq!(out.cell(0, 0).expect("wide cell").text(), "\u{4e2d}");
        assert_eq!(out.cell(2, 0).expect("padding cell").text(), " ");
        assert_eq!(out.cell(3, 0).expect("padding cell").text(), " ");
    }

    #[test]
    fn clips_wide_graphemes_at_the_display_boundary() {
        let mut out = Surface::new(4, 1, Color::White, Color::Black);
        write_padded(&mut out, "a\u{4e2d}\u{4e2d}", 4).expect("write padded row");
        assert_eq!(out.cell(0, 0).expect("cell").text(), "a");
        assert_eq!(out.cell(1, 0).expect("wide cell").text(), "\u{4e2d}");
        assert_eq!(out.cell(3, 0).expect("padding cell").text(), " ");
    }
}
