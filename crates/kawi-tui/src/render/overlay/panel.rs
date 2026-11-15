use anyhow::{Context, Result};
use crossterm::style::Color;
use std::io::Write;

use crate::frame::{DecorationStyle, Frame, OverlayPanel};
use crate::markdown::{MarkdownLine, MarkdownStyle};
use crate::overlay::{PANEL_BORDER, PANEL_PADDING};
use crate::paint;
use crate::surface::Surface;
use crate::text::{display_width, truncate_to_display_width};
use crate::theme::Theme;

use super::super::style::apply_markdown_style;
use super::draw_rounded_box;

pub(super) fn draw(out: &mut Surface, frame: &Frame, panel: &OverlayPanel<'_>) -> Result<()> {
    super::clear_overlay_rect(out, panel.rect, frame.theme)?;
    draw_rounded_box(
        out,
        panel.rect,
        frame.theme,
        accent_color(frame.theme, panel.focused),
    )?;
    let width = overlay_content_width(panel.rect.width);
    let height = panel.rect.height.saturating_sub(PANEL_BORDER);
    if width == 0 || height == 0 {
        return Ok(());
    }
    for offset in 0..height {
        draw_overlay_row(out, frame, panel, offset)?;
    }
    draw_scrollbar(out, frame.theme, panel)?;
    draw_caption(out, frame, panel)
}

fn accent_color(theme: &Theme, focused: bool) -> Color {
    if focused {
        theme.decoration_colors(DecorationStyle::Accent).foreground
    } else {
        theme.non_text
    }
}

struct Caption<'a> {
    text: &'a str,
    row: usize,
    width: usize,
}

fn draw_caption(out: &mut Surface, frame: &Frame, panel: &OverlayPanel<'_>) -> Result<()> {
    if panel.rect.width < 5 {
        return Ok(());
    }
    let width = panel.rect.width.saturating_sub(4);
    if let Some(title) = panel.title.filter(|title| !title.is_empty()) {
        draw_caption_line(
            out,
            frame,
            panel,
            Caption {
                text: title,
                row: panel.rect.y,
                width,
            },
        )?;
    }
    if let Some(status) = panel.status.filter(|status| !status.is_empty()) {
        draw_caption_line(
            out,
            frame,
            panel,
            Caption {
                text: status,
                row: panel.rect.bottom().saturating_sub(1),
                width,
            },
        )?;
    }
    Ok(())
}

fn draw_caption_line(
    out: &mut Surface,
    frame: &Frame,
    panel: &OverlayPanel<'_>,
    caption: Caption<'_>,
) -> Result<()> {
    let text = truncate_to_display_width(caption.text, caption.width.saturating_sub(2));
    if text.is_empty() {
        return Ok(());
    }
    let foreground = accent_color(frame.theme, panel.focused);
    out.move_to(panel.rect.x.saturating_add(2), caption.row);
    paint::apply_colors(out, foreground, frame.theme.float_background);
    write!(out, " {text} ").context("write overlay caption")?;
    paint::restore_base(out, frame.theme);
    Ok(())
}

#[derive(Clone, Copy)]
struct RowColors {
    foreground: Color,
    background: Color,
}

struct RowStyle<'a> {
    theme: &'a Theme,
    colors: RowColors,
}

fn draw_overlay_row(
    out: &mut Surface,
    frame: &Frame,
    panel: &OverlayPanel<'_>,
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
    let selected = panel.selected == Some(index);
    let line = panel.lines.get(index);
    let colors = row_colors(frame.theme, selected, line);
    let used = match line {
        Some(line) => draw_markdown_line(
            out,
            line,
            width,
            RowStyle {
                theme: frame.theme,
                colors,
            },
        )?,
        None => 0,
    };
    if used < width {
        paint::apply_colors(out, colors.foreground, colors.background);
        write!(out, "{}", " ".repeat(width.saturating_sub(used)))
            .context("pad overlay panel row")?;
    }
    paint::restore_base(out, frame.theme);
    Ok(())
}

fn row_colors(theme: &Theme, selected: bool, line: Option<&MarkdownLine>) -> RowColors {
    if selected {
        return RowColors {
            foreground: theme.selection.foreground,
            background: theme.selection.background,
        };
    }
    RowColors {
        foreground: theme.base.foreground,
        background: line.map_or(theme.float_background, |line| {
            overlay_background(line, theme)
        }),
    }
}

fn overlay_content_width(panel_width: usize) -> usize {
    panel_width
        .saturating_sub(PANEL_BORDER)
        .saturating_sub(2 * PANEL_PADDING)
}

fn draw_scrollbar(out: &mut Surface, theme: &Theme, panel: &OverlayPanel<'_>) -> Result<()> {
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

pub(in crate::render) fn scrollbar_thumb(
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

pub(in crate::render) fn overlay_background(line: &MarkdownLine, theme: &Theme) -> Color {
    if line.code {
        theme.code_background
    } else {
        theme.float_background
    }
}

fn draw_markdown_line(
    out: &mut Surface,
    line: &MarkdownLine,
    width: usize,
    style: RowStyle<'_>,
) -> Result<usize> {
    let mut used = 0;
    for span in &line.spans {
        if used >= width {
            break;
        }
        let text = truncate_to_display_width(&span.text, width.saturating_sub(used));
        if text.is_empty() {
            continue;
        }
        if span.style == MarkdownStyle::Normal {
            paint::apply_colors(out, style.colors.foreground, style.colors.background);
        } else {
            apply_markdown_style(out, span.style, style.colors.background, style.theme)?;
        }
        write!(out, "{text}").context("write overlay span")?;
        used = used.saturating_add(display_width(text));
    }
    out.reset_attributes();
    paint::apply_base(out, style.theme);
    Ok(used)
}
