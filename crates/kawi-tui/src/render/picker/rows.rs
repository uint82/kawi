use anyhow::{Context, Result};
use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Attribute, SetAttribute, SetForegroundColor},
};
use std::io::{Stdout, Write};

use super::super::{picker_start, saturate_u16};
use super::write_padding;
use crate::frame::{Frame, PickerLine, PickerMenu, Rect};
use crate::paint;
use crate::text::{count_label, display_width, truncate_to_display_width};

pub(super) fn draw_results(
    out: &mut Stdout,
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
        execute!(out, MoveTo(saturate_u16(rect.x), saturate_u16(row)))
            .context("move to picker result")?;
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
    out: &mut Stdout,
    frame: &Frame,
    item: &PickerLine<'_>,
    width: usize,
    selected: bool,
) -> Result<()> {
    apply_result_colors(out, frame, selected)?;
    if item.deprecated {
        execute!(out, SetAttribute(Attribute::CrossedOut)).context("cross out picker row")?;
    }
    let primary = truncate_to_display_width(item.primary, width);
    write!(out, "{primary}").context("write picker result")?;
    if item.deprecated {
        execute!(out, SetAttribute(Attribute::NotCrossedOut)).context("uncross picker row")?;
    }
    let used = write_detail(out, frame, item, width, display_width(primary))?;
    write_padding(out, used, width)
}

fn apply_result_colors(out: &mut Stdout, frame: &Frame, selected: bool) -> Result<()> {
    if selected {
        paint::apply_match(out, &frame.theme.selection)?;
        return Ok(());
    }
    paint::apply_colors(
        out,
        frame.theme.base.foreground,
        frame.theme.float_background,
        "apply picker result colors",
    )
}

fn write_detail(
    out: &mut Stdout,
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
    execute!(out, SetForegroundColor(frame.theme.line_number))
        .context("apply picker result detail color")?;
    let detail = truncate_to_display_width(detail, remaining);
    write!(out, "{detail}").context("write picker result detail")?;
    Ok(used.saturating_add(display_width(detail)))
}

pub(super) fn draw_footer(
    out: &mut Stdout,
    frame: &Frame,
    picker: &PickerMenu<'_>,
    rect: Rect,
) -> Result<()> {
    if rect.width == 0 || rect.height == 0 {
        return Ok(());
    }
    execute!(out, MoveTo(saturate_u16(rect.x), saturate_u16(rect.y)))
        .context("move to picker footer")?;
    paint::apply_match(out, &frame.theme.status)?;
    let footer = picker_footer(picker);
    let footer = truncate_to_display_width(&footer, rect.width);
    write!(out, "{footer}").context("write picker footer")?;
    write_padding(out, display_width(footer), rect.width)?;
    paint::apply_colors(
        out,
        frame.theme.base.foreground,
        frame.theme.float_background,
        "restore picker footer colors",
    )
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

fn fill_row(out: &mut Stdout, frame: &Frame, width: usize) -> Result<()> {
    paint::apply_colors(
        out,
        frame.theme.base.foreground,
        frame.theme.float_background,
        "apply picker row background",
    )?;
    write!(out, "{}", " ".repeat(width)).context("fill picker row")?;
    Ok(())
}
