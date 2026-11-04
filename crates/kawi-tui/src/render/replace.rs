use anyhow::{Context, Result};
use crossterm::{cursor::MoveTo, execute};
use std::io::{Stdout, Write};

use crate::display;
use crate::frame::{Frame, ReplaceMenu, ReplaceStage};
use crate::paint;
use crate::surface::{saturate_u16, Surface};
use crate::text::{
    count_label, display_width, sanitize_text, truncate_to_display_width, TAB_WIDTH,
};

use super::picker_start;

const SEARCH_LABEL: &str = "Replace  ";
const REPLACEMENT_LABEL: &str = "with: ";
const SEARCH_ROW: usize = 0;
const REPLACEMENT_ROW: usize = 1;

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
    write_replace_prompt(
        out,
        frame,
        SEARCH_ROW,
        query_prompt(SEARCH_LABEL, replace.query),
    )?;
    if replace.stage == ReplaceStage::Search {
        paint::restore_base(out, frame.theme);
        return Ok(());
    }
    write_replace_prompt(
        out,
        frame,
        REPLACEMENT_ROW,
        query_prompt(REPLACEMENT_LABEL, replace.replacement),
    )?;
    paint::restore_base(out, frame.theme);
    Ok(())
}

fn write_replace_prompt(
    out: &mut Surface,
    frame: &Frame,
    row: usize,
    prompt: String,
) -> Result<()> {
    out.move_to(0, row);
    out.clear_line();
    out.set_foreground(frame.theme.line_number);
    write!(
        out,
        "{}",
        truncate_to_display_width(&prompt, frame.viewport.width())
    )
    .context("write replace prompt")?;
    Ok(())
}

fn query_prompt(label: &str, text: &str) -> String {
    format!("{label}{}", sanitize_text(text))
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
        write!(out, "{}", truncate_to_display_width(&item, width)).context("write replace row")?;
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
        truncate_to_display_width(&footer, frame.viewport.width())
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
    let position = replace_cursor_position(replace, frame.viewport.width().max(1));
    execute!(
        out,
        MoveTo(saturate_u16(position.column), saturate_u16(position.row))
    )
    .context("place replace cursor")?;
    Ok(())
}

struct ReplaceCursorPosition {
    row: usize,
    column: usize,
}

fn replace_cursor_position(replace: &ReplaceMenu<'_>, width: usize) -> ReplaceCursorPosition {
    let (row, label, text) = match replace.stage {
        ReplaceStage::Search => (SEARCH_ROW, SEARCH_LABEL, replace.query),
        ReplaceStage::Replacement | ReplaceStage::Preview => {
            (REPLACEMENT_ROW, REPLACEMENT_LABEL, replace.replacement)
        }
    };
    let prefix = display_width(label);
    let text = display::screen_col(text, replace.cursor, TAB_WIDTH);
    ReplaceCursorPosition {
        row,
        column: prefix.saturating_add(text).min(width.saturating_sub(1)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu<'a>(stage: ReplaceStage, query: &'a str, replacement: &'a str) -> ReplaceMenu<'a> {
        ReplaceMenu {
            stage,
            query,
            replacement,
            cursor: 1,
            items: &[],
            selected: None,
            loading: false,
            status: None,
            matches: 0,
            files: 0,
        }
    }

    #[test]
    fn cursor_positions_account_for_wide_text() {
        let replace = menu(ReplaceStage::Search, "\u{4e2d}x", "");
        let position = replace_cursor_position(&replace, 30);
        assert_eq!(position.row, SEARCH_ROW);
        assert_eq!(position.column, display_width(SEARCH_LABEL) + 2);
    }

    #[test]
    fn cursor_positions_clamp_to_the_viewport() {
        let replace = menu(ReplaceStage::Replacement, "", "abcdef");
        let position = replace_cursor_position(&replace, 5);
        assert_eq!(position.row, REPLACEMENT_ROW);
        assert_eq!(position.column, 4);
    }
}
