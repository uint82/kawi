use anyhow::{Context, Result};
use crossterm::style::Color;
use std::io::Write;

use crate::frame::Frame;
use crate::overlay::{PANEL_BORDER, PANEL_PADDING};
use crate::paint;
use crate::surface::Surface;
use crate::text::{display_width, truncate_to_display_width};
use crate::theme::Theme;

use super::completion_row;
use super::furniture::write_padded;
use super::keymenu;
use super::menu;
use super::style::apply_markdown_style;

pub(super) fn draw_overlay(out: &mut Surface, frame: &Frame) -> Result<()> {
    let Some(overlay) = frame.overlay.as_ref() else {
        return Ok(());
    };
    if let Some(list) = overlay.list.as_ref() {
        draw_overlay_list(out, frame, list)?;
    }
    if let Some(panel) = overlay.panel.as_ref() {
        draw_overlay_panel(out, frame, panel)?;
    }
    if let Some(menu) = overlay.key_menu.as_ref() {
        keymenu::draw(out, frame, menu)?;
    }
    Ok(())
}

fn draw_overlay_list(
    out: &mut Surface,
    frame: &Frame,
    list: &crate::frame::OverlayList<'_>,
) -> Result<()> {
    clear_overlay_rect(out, list.rect, frame.theme)?;
    draw_rounded_box(out, list.rect, frame.theme)?;
    let width = list.rect.width.saturating_sub(2);
    let height = list.rect.height.saturating_sub(2);
    if width == 0 || height == 0 {
        return Ok(());
    }
    let start = menu::visible_start(height, list.selected);
    for offset in 0..height {
        let index = start.saturating_add(offset);
        let row = list.rect.y.saturating_add(offset).saturating_add(1);
        out.move_to(list.rect.x.saturating_add(1), row);
        let selected = list.selected == Some(index);
        if let Some(line) = list.items.get(index) {
            completion_row::draw_row(out, frame.theme, line, width, list.kind_width, selected)?;
        } else {
            completion_row::draw_blank(out, frame.theme, width, selected)?;
        }
    }
    Ok(())
}

fn draw_overlay_panel(
    out: &mut Surface,
    frame: &Frame,
    panel: &crate::frame::OverlayPanel<'_>,
) -> Result<()> {
    clear_overlay_rect(out, panel.rect, frame.theme)?;
    draw_rounded_box(out, panel.rect, frame.theme)?;
    let width = overlay_content_width(panel.rect.width);
    let height = panel.rect.height.saturating_sub(PANEL_BORDER);
    if width == 0 || height == 0 {
        return Ok(());
    }
    for offset in 0..height {
        draw_overlay_row(out, frame, panel, offset)?;
    }
    draw_scrollbar(out, frame.theme, panel)?;
    Ok(())
}

fn draw_overlay_row(
    out: &mut Surface,
    frame: &Frame,
    panel: &crate::frame::OverlayPanel<'_>,
    offset: usize,
) -> Result<()> {
    let width = overlay_content_width(panel.rect.width);
    let index = panel.scroll.saturating_add(offset);
    let row = panel.rect.y.saturating_add(offset).saturating_add(1);
    let x = panel
        .rect
        .x
        .saturating_add(PANEL_BORDER / 2)
        .saturating_add(PANEL_PADDING);
    out.move_to(x, row);
    let (used, background) = match panel.lines.get(index) {
        Some(line) => (
            draw_markdown_line(out, line, width, frame.theme)?,
            overlay_background(line, frame.theme),
        ),
        None => (0, frame.theme.float_background),
    };
    if used < width {
        paint::apply_colors(out, frame.theme.base.foreground, background);
        write!(out, "{}", " ".repeat(width.saturating_sub(used)))
            .context("pad overlay panel row")?;
    }
    paint::restore_base(out, frame.theme);
    Ok(())
}

fn overlay_content_width(panel_width: usize) -> usize {
    panel_width
        .saturating_sub(PANEL_BORDER)
        .saturating_sub(2 * PANEL_PADDING)
}

fn draw_scrollbar(
    out: &mut Surface,
    theme: &Theme,
    panel: &crate::frame::OverlayPanel<'_>,
) -> Result<()> {
    let visible = panel.rect.height.saturating_sub(PANEL_BORDER);
    let Some(thumb) = scrollbar_thumb(panel.scroll, panel.lines.len(), visible) else {
        return Ok(());
    };
    paint::apply_colors(out, theme.non_text, theme.float_background);
    let x = panel.rect.right().saturating_sub(1);
    for offset in 0..visible {
        let row = panel.rect.y.saturating_add(offset).saturating_add(1);
        out.move_to(x, row);
        let glyph = if thumb.contains(&offset) {
            '█'
        } else {
            '│'
        };
        write!(out, "{glyph}").context("write overlay scrollbar")?;
    }
    paint::restore_base(out, theme);
    Ok(())
}

pub(super) fn scrollbar_thumb(
    scroll: usize,
    total: usize,
    visible: usize,
) -> Option<std::ops::Range<usize>> {
    if visible == 0 || total <= visible {
        return None;
    }
    let thumb = visible
        .saturating_mul(visible)
        .checked_div(total)
        .unwrap_or(0)
        .max(1)
        .min(visible);
    let travel = visible.saturating_sub(thumb);
    let max_scroll = total.saturating_sub(visible);
    let start = scroll
        .min(max_scroll)
        .saturating_mul(travel)
        .checked_div(max_scroll)
        .unwrap_or(0);
    Some(start..start.saturating_add(thumb))
}

pub(super) fn overlay_background(line: &crate::markdown::MarkdownLine, theme: &Theme) -> Color {
    if line.code {
        theme.code_background
    } else {
        theme.float_background
    }
}

fn draw_markdown_line(
    out: &mut Surface,
    line: &crate::markdown::MarkdownLine,
    width: usize,
    theme: &Theme,
) -> Result<usize> {
    let mut used = 0;
    let background = overlay_background(line, theme);
    for span in &line.spans {
        if used >= width {
            break;
        }
        let text = truncate_to_display_width(&span.text, width.saturating_sub(used));
        if text.is_empty() {
            continue;
        }
        apply_markdown_style(out, span.style, background, theme)?;
        write!(out, "{text}").context("write overlay span")?;
        used = used.saturating_add(display_width(text));
    }
    out.reset_attributes();
    paint::apply_base(out, theme);
    Ok(used)
}

pub(super) fn clear_overlay_rect(
    out: &mut Surface,
    rect: crate::frame::Rect,
    theme: &Theme,
) -> Result<()> {
    if rect.width == 0 || rect.height == 0 {
        return Ok(());
    }
    paint::apply_colors(out, theme.base.foreground, theme.float_background);
    for row in rect.y..rect.bottom() {
        out.move_to(rect.x, row);
        write_padded(out, "", rect.width)?;
    }
    Ok(())
}

pub(super) fn draw_rounded_box(
    out: &mut Surface,
    rect: crate::frame::Rect,
    theme: &Theme,
) -> Result<()> {
    if rect.width < 2 || rect.height < 2 {
        return Ok(());
    }
    out.set_foreground(theme.non_text);
    let inner = rect.width.saturating_sub(2);
    let top = format!("╭{}╮", "─".repeat(inner));
    let bottom = format!("╰{}╯", "─".repeat(inner));
    out.move_to(rect.x, rect.y);
    write!(out, "{top}").context("write overlay top border")?;
    out.move_to(rect.x, rect.bottom().saturating_sub(1));
    write!(out, "{bottom}").context("write overlay bottom border")?;
    for row in rect.y.saturating_add(1)..rect.bottom().saturating_sub(1) {
        out.move_to(rect.x, row);
        write!(out, "│").context("write overlay left border")?;
        out.move_to(rect.right().saturating_sub(1), row);
        write!(out, "│").context("write overlay right border")?;
    }
    paint::restore_base(out, theme);
    Ok(())
}
