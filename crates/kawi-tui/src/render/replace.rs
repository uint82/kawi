use anyhow::{Context, Result};
use crossterm::{cursor::MoveTo, execute};
use std::io::{Stdout, Write};

use crate::frame::{Frame, ReplaceMenu, ReplaceStage};
use crate::paint;
use crate::surface::Surface;
use crate::text::{count_label, sanitize_text};

use super::furniture::saturate_u16;

use super::furniture::truncate_to_width;
use super::picker_start;

pub(super) fn draw_replace(
    out: &mut Surface,
    frame: &Frame,
    replace: &ReplaceMenu<'_>,
) -> Result<()> {
    paint::apply_base(out, frame.theme);
    out.clear_all();
    draw_replace_prompt(out, frame, replace)?;
    draw_replace_rows(out, frame, replace)?;
    draw_replace_footer(out, frame, replace)?;
    Ok(())
}

fn draw_replace_prompt(out: &mut Surface, frame: &Frame, replace: &ReplaceMenu<'_>) -> Result<()> {
    out.move_to(0, 0);
    out.clear_line();
    out.set_foreground(frame.theme.line_number);
    let query = sanitize_text(replace.query);
    let prompt = format!("Replace  {query}");
    write!(
        out,
        "{}",
        truncate_to_width(&prompt, frame.viewport.width())
    )
    .context("write replace prompt")?;
    if replace.stage == ReplaceStage::Search {
        paint::restore_base(out, frame.theme);
        return Ok(());
    }
    out.move_to(0, 1);
    out.clear_line();
    out.set_foreground(frame.theme.line_number);
    let replacement = sanitize_text(replace.replacement);
    let prompt = format!("with: {replacement}");
    write!(
        out,
        "{}",
        truncate_to_width(&prompt, frame.viewport.width())
    )
    .context("write replacement prompt")?;
    paint::restore_base(out, frame.theme);
    Ok(())
}

fn draw_replace_rows(out: &mut Surface, frame: &Frame, replace: &ReplaceMenu<'_>) -> Result<()> {
    let first_row = replace_rows_start(replace.stage);
    let available = frame.viewport.height().saturating_sub(first_row + 1);
    if available == 0 {
        return Ok(());
    }
    let start = picker_start(available, replace.selected);
    let width = frame.viewport.width();
    for offset in 0..available {
        let index = start + offset;
        let row = first_row + offset;
        out.move_to(0, row);
        out.clear_line();
        let Some(item) = replace.items.get(index) else {
            continue;
        };
        if replace.selected == Some(index) {
            paint::apply_match(out, &frame.theme.selection);
        }
        let item = sanitize_text(item);
        write!(out, "{}", truncate_to_width(&item, width)).context("write replace row")?;
        paint::restore_base(out, frame.theme);
    }
    Ok(())
}

fn draw_replace_footer(out: &mut Surface, frame: &Frame, replace: &ReplaceMenu<'_>) -> Result<()> {
    let height = frame.viewport.height();
    if height == 0 {
        return Ok(());
    }
    let row = height.saturating_sub(1);
    out.move_to(0, row);
    out.clear_line();
    let footer = replace_footer(replace);
    paint::apply_match(out, &frame.theme.status);
    write!(
        out,
        "{}",
        truncate_to_width(&footer, frame.viewport.width())
    )
    .context("write replace footer")?;
    paint::restore_base(out, frame.theme);
    Ok(())
}

fn replace_footer(replace: &ReplaceMenu<'_>) -> String {
    if let Some(status) = replace.status {
        return status.to_string();
    }
    if replace.loading {
        return match replace.stage {
            ReplaceStage::Search => "Searching...".to_string(),
            ReplaceStage::Replacement | ReplaceStage::Preview => "Building preview...".to_string(),
        };
    }
    match replace.stage {
        ReplaceStage::Search => {
            if replace.items.is_empty() {
                return "Type to search  Enter replace  Esc cancel".to_string();
            }
            format!(
                "{} results  Enter replace  Tab next  Esc cancel",
                replace.items.len()
            )
        }
        ReplaceStage::Replacement => "Enter preview  Esc search  groups: & \\1 $1".to_string(),
        ReplaceStage::Preview => {
            let matches = count_label(replace.matches, "match", "matches");
            let files = count_label(replace.files, "file", "files");
            format!("{matches} in {files}  y apply  n/Esc back")
        }
    }
}

fn replace_rows_start(stage: ReplaceStage) -> usize {
    match stage {
        ReplaceStage::Search => 1,
        ReplaceStage::Replacement | ReplaceStage::Preview => 2,
    }
}

pub(super) fn place_replace_cursor(
    out: &mut Stdout,
    frame: &Frame,
    replace: &ReplaceMenu<'_>,
) -> Result<()> {
    let width = frame.viewport.width().max(1);
    let (row, prefix) = match replace.stage {
        ReplaceStage::Search => (0, "Replace  ".chars().count()),
        ReplaceStage::Replacement | ReplaceStage::Preview => (1, "with: ".chars().count()),
    };
    let column = prefix.saturating_add(replace.cursor).min(width - 1);
    execute!(out, MoveTo(saturate_u16(column), saturate_u16(row)))
        .context("place replace cursor")?;
    Ok(())
}
