use anyhow::{Context, Result};
use std::io::Write;

use super::super::picker_start;
use super::write_padding;
use crate::frame::{Frame, PickerLine, PickerMenu, Rect};
use crate::paint;
use crate::surface::Surface;
use crate::text::{count_label, display_width, sanitize_text, truncate_to_display_width};

pub(super) fn draw_results(
    out: &mut Surface,
    frame: &Frame,
    picker: &PickerMenu<'_>,
    rect: Rect,
) -> Result<()> {
    if rect.height == 0 {
        return Ok(());
    }
    let start = picker_start(rect.height, picker.selected);
    for offset in 0..rect.height {
        let row = rect.y.saturating_add(offset);
        out.move_to(rect.x, row);
        let index = start.saturating_add(offset);
        let Some(item) = picker.items.get(index) else {
            fill_row(out, frame, rect.width)?;
            continue;
        };
        draw_result(out, frame, item, rect.width, picker.selected == Some(index))?;
    }
    Ok(())
}

fn draw_result(
    out: &mut Surface,
    frame: &Frame,
    item: &PickerLine<'_>,
    width: usize,
    selected: bool,
) -> Result<()> {
    apply_result_colors(out, frame, selected);
    out.set_crossed_out(item.deprecated);
    let primary = sanitize_text(item.primary);
    let primary = truncate_to_display_width(&primary, width);
    write!(out, "{primary}").context("write picker result")?;
    out.set_crossed_out(false);
    let used = write_detail(out, frame, item, width, display_width(primary))?;
    write_padding(out, used, width)
}

fn apply_result_colors(out: &mut Surface, frame: &Frame, selected: bool) {
    if selected {
        paint::apply_match(out, &frame.theme.selection);
        return;
    }
    paint::apply_colors(
        out,
        frame.theme.base.foreground,
        frame.theme.float_background,
    );
}

fn write_detail(
    out: &mut Surface,
    frame: &Frame,
    item: &PickerLine<'_>,
    width: usize,
    used: usize,
) -> Result<usize> {
    let Some(detail) = item.detail.filter(|detail| !detail.is_empty()) else {
        return Ok(used);
    };
    let remaining = width.saturating_sub(used);
    if remaining == 0 {
        return Ok(used);
    }
    out.set_foreground(frame.theme.line_number);
    let detail = sanitize_text(detail);
    let detail = truncate_to_display_width(&detail, remaining);
    write!(out, "{detail}").context("write picker result detail")?;
    Ok(used.saturating_add(display_width(detail)))
}

pub(super) fn draw_footer(
    out: &mut Surface,
    frame: &Frame,
    picker: &PickerMenu<'_>,
    rect: Rect,
) -> Result<()> {
    if rect.width == 0 || rect.height == 0 {
        return Ok(());
    }
    out.move_to(rect.x, rect.y);
    paint::apply_match(out, &frame.theme.status);
    let footer = picker_footer(picker);
    let footer = truncate_to_display_width(&footer, rect.width);
    write!(out, "{footer}").context("write picker footer")?;
    write_padding(out, display_width(footer), rect.width)?;
    paint::apply_colors(
        out,
        frame.theme.base.foreground,
        frame.theme.float_background,
    );
    Ok(())
}

fn picker_footer(picker: &PickerMenu<'_>) -> String {
    if let Some(status) = picker.status {
        return status.to_string();
    }
    if picker.loading {
        return "Searching...".to_string();
    }
    if picker.query.is_empty() && picker.items.is_empty() {
        return "Type to search  Esc cancel".to_string();
    }
    if picker.items.is_empty() {
        return "No results  Esc cancel".to_string();
    }
    let results = count_label(picker.items.len(), "result", "results");
    if picker.replace_hint {
        return format!("{results}  Enter open  Ctrl-R replace  Tab next  Esc cancel");
    }
    if picker.file_hint {
        return format!(
            "{results}  Enter open  Ctrl-A new  Ctrl-R rename  Ctrl-D delete  Tab next  Esc cancel"
        );
    }
    format!("{results}  Enter open  Tab next  Esc cancel")
}

fn fill_row(out: &mut Surface, frame: &Frame, width: usize) -> Result<()> {
    paint::apply_colors(
        out,
        frame.theme.base.foreground,
        frame.theme.float_background,
    );
    write!(out, "{}", " ".repeat(width)).context("fill picker row")?;
    Ok(())
}
