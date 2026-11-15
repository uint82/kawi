use anyhow::{Context, Result};
use crossterm::style::Color;
use std::io::Write;

use crate::frame::Frame;
use crate::paint;
use crate::surface::Surface;
use crate::theme::Theme;

use super::completion_row;
use super::furniture::write_padded;
use super::keymenu;
use super::menu;

mod panel;

#[cfg(test)]
pub(super) use panel::{overlay_background, scrollbar_thumb};

pub(super) fn draw_overlay(out: &mut Surface, frame: &Frame) -> Result<()> {
    let Some(overlay) = frame.overlay.as_ref() else {
        return Ok(());
    };
    if let Some(list) = overlay.list.as_ref() {
        draw_overlay_list(out, frame, list)?;
    }
    if let Some(panel) = overlay.panel.as_ref() {
        panel::draw(out, frame, panel)?;
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
    draw_rounded_box(out, list.rect, frame.theme, frame.theme.non_text)?;
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
    color: Color,
) -> Result<()> {
    if rect.width < 2 || rect.height < 2 {
        return Ok(());
    }
    out.set_foreground(color);
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
