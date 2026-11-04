use anyhow::{Context, Result};
use crossterm::style::Color;
use std::io::Write;

use crate::frame::{PaneFrame, VirtualTextKind, VirtualTextPlacement};
use crate::paint;
use crate::surface::Surface;
use crate::text::{display_width, truncate_to_display_width};
use crate::theme::Theme;

use super::furniture::write_padded;

const VIRTUAL_TEXT_GAP: usize = 1;
const VIRTUAL_TEXT_MIN_COLUMNS: usize = 10;

pub(super) struct VirtualTextDraw<'a> {
    pub(super) pane: &'a PaneFrame<'a>,
    pub(super) theme: &'a Theme,
    pub(super) screen_row: usize,
    pub(super) gutter_width: usize,
    pub(super) width: usize,
    pub(super) line_width: usize,
}

pub(super) fn draw_left_virtual_text(
    out: &mut Surface,
    pane: &PaneFrame<'_>,
    theme: &Theme,
    screen_row: usize,
) -> Result<()> {
    if pane.left_width == 0 {
        return Ok(());
    }
    let text = pane.virtual_texts.get(screen_row).and_then(|texts| {
        texts
            .iter()
            .find(|text| text.placement == VirtualTextPlacement::BeforeLine)
    });
    if let Some(text) = text {
        paint::apply_faded_text(out, virtual_text_color(theme, text.kind));
        write_padded(out, &text.text, pane.left_width)?;
    } else {
        write!(out, "{}", " ".repeat(pane.left_width)).context("write empty prefix")?;
    }
    finish_virtual_text(out, theme)
}

pub(super) fn draw_virtual_text(out: &mut Surface, draw: VirtualTextDraw<'_>) -> Result<()> {
    let primary = draw
        .pane
        .virtual_texts
        .get(draw.screen_row)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let plugin = draw
        .pane
        .plugin_virtual_texts
        .get(draw.screen_row)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let segments = virtual_text_segments(draw.line_width, draw.width, primary.iter().chain(plugin));
    for segment in segments {
        out.move_to(
            draw.pane
                .rect
                .x
                .saturating_add(draw.gutter_width)
                .saturating_add(segment.column),
            draw.pane.rect.y.saturating_add(draw.screen_row),
        );
        paint::apply_faded_text(out, virtual_text_color(draw.theme, segment.text.kind));
        write!(out, "{}", segment.clipped).context("write virtual text")?;
    }
    finish_virtual_text(out, draw.theme)
}

pub(super) struct VirtualTextSegment<'a> {
    pub(super) column: usize,
    pub(super) text: &'a crate::frame::VirtualText,
    pub(super) clipped: &'a str,
}

pub(super) fn virtual_text_segments<'a>(
    line_width: usize,
    pane_width: usize,
    texts: impl IntoIterator<Item = &'a crate::frame::VirtualText>,
) -> Vec<VirtualTextSegment<'a>> {
    let remaining = pane_width.saturating_sub(line_width);
    if remaining < VIRTUAL_TEXT_MIN_COLUMNS {
        return Vec::new();
    }
    let mut column = line_width.saturating_add(VIRTUAL_TEXT_GAP);
    let mut segments = Vec::new();
    for text in texts {
        if text.placement == VirtualTextPlacement::BeforeLine || column >= pane_width {
            continue;
        }
        let clipped = truncate_to_display_width(&text.text, pane_width - column);
        if clipped.is_empty() {
            continue;
        }
        segments.push(VirtualTextSegment {
            column,
            text,
            clipped,
        });
        column = column
            .saturating_add(display_width(clipped))
            .saturating_add(VIRTUAL_TEXT_GAP);
    }
    segments
}

pub(super) fn finish_virtual_text(out: &mut Surface, theme: &Theme) -> Result<()> {
    paint::clear_decorations(out);
    paint::restore_base(out, theme);
    Ok(())
}

fn virtual_text_color(theme: &Theme, kind: VirtualTextKind) -> Color {
    match kind {
        VirtualTextKind::Diagnostic(severity) => theme.diagnostic_color(severity),
        VirtualTextKind::Plugin(style) => style.accent,
        VirtualTextKind::Metadata => theme.non_text,
    }
}
