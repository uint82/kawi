mod chrome;
mod preview;
mod rows;

use anyhow::{Context, Result};
use crossterm::{cursor::MoveTo, execute};
use std::io::{Stdout, Write};

use self::chrome::{draw_box, draw_divider, draw_rule};
use self::preview::draw_preview;
use self::rows::{draw_footer, draw_results};
use super::draw_surface;
use super::furniture::saturate_u16;
use super::overlay::clear_overlay_rect;
use crate::display;
use crate::frame::{Frame, PickerMenu, Rect};
use crate::paint;
use crate::picker_layout;
use crate::surface::Surface;
use crate::text::display_width;

const PROMPT_PREFIX: &str = "> ";
const PROMPT_PREFIX_WIDTH: usize = 2;
const QUERY_TABSTOP: usize = 4;

pub(super) fn draw(surface: &mut Surface, frame: &Frame, picker: &PickerMenu<'_>) -> Result<()> {
    draw_surface(surface, frame)?;
    let layout = picker_layout::layout(picker_area(frame), picker.preview_enabled);
    if layout.prompt.width <= PROMPT_PREFIX_WIDTH || layout.prompt.height == 0 {
        return Ok(());
    }
    clear_overlay_rect(surface, layout.frame, frame.theme)?;
    draw_box(surface, frame, layout.frame, picker.title)?;
    draw_prompt(surface, frame, picker, layout.prompt)?;
    draw_rule(surface, frame, layout.rule)?;
    draw_results(surface, frame, picker, layout.results)?;
    draw_divider(surface, frame, layout.divider)?;
    draw_preview(surface, frame, picker.preview.as_ref(), layout.preview)?;
    draw_footer(surface, frame, picker, layout.footer)?;
    Ok(())
}

pub(super) fn place_cursor(out: &mut Stdout, frame: &Frame, picker: &PickerMenu<'_>) -> Result<()> {
    let layout = picker_layout::layout(picker_area(frame), picker.preview_enabled);
    place_prompt_cursor(out, picker, layout.prompt)
}

fn picker_area(frame: &Frame) -> Rect {
    Rect::new(
        0,
        0,
        frame.viewport.width().max(1),
        frame.viewport.height().saturating_add(1).max(1),
    )
}

fn draw_prompt(
    out: &mut Surface,
    frame: &Frame,
    picker: &PickerMenu<'_>,
    rect: Rect,
) -> Result<()> {
    out.move_to(rect.x, rect.y);
    out.set_foreground(frame.theme.non_text);
    write!(out, "{PROMPT_PREFIX}").context("write picker prompt marker")?;
    paint::apply_colors(
        out,
        frame.theme.base.foreground,
        frame.theme.float_background,
    );
    let view = prompt_view(picker.query, picker.cursor, rect.width);
    write!(out, "{}", view.text).context("write picker prompt")?;
    write_padding(
        out,
        view.text_width,
        rect.width.saturating_sub(PROMPT_PREFIX_WIDTH),
    )
}

struct PromptView {
    text: String,
    text_width: usize,
    cursor: usize,
}

fn prompt_view(query: &str, cursor: usize, width: usize) -> PromptView {
    let available = width.saturating_sub(PROMPT_PREFIX_WIDTH);
    let prefix: String = query.chars().take(cursor).collect();
    let cursor_column = display_width(&prefix);
    let offset = cursor_column
        .saturating_add(1)
        .saturating_sub(available)
        .min(cursor_column);
    let text = display::slice_screen(
        query,
        offset,
        offset.saturating_add(available),
        QUERY_TABSTOP,
    );
    PromptView {
        text_width: display_width(&text),
        cursor: cursor_column.saturating_sub(offset),
        text,
    }
}

fn place_prompt_cursor(out: &mut Stdout, picker: &PickerMenu<'_>, rect: Rect) -> Result<()> {
    let view = prompt_view(picker.query, picker.cursor, rect.width);
    let column = rect
        .x
        .saturating_add(PROMPT_PREFIX_WIDTH)
        .saturating_add(view.cursor)
        .min(rect.right().saturating_sub(1));
    execute!(out, MoveTo(saturate_u16(column), saturate_u16(rect.y)))
        .context("place picker cursor")?;
    Ok(())
}

pub(super) fn write_padding(out: &mut Surface, used: usize, width: usize) -> Result<()> {
    let padding = width.saturating_sub(used);
    if padding == 0 {
        return Ok(());
    }
    write!(out, "{}", " ".repeat(padding)).context("pad picker row")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_scrolls_to_keep_the_cursor_visible() {
        let view = prompt_view("abcdefghij", 10, 8);
        assert_eq!(view.text, "fghij");
        assert_eq!(view.cursor, 5);
    }

    #[test]
    fn prompt_keeps_the_cursor_in_view_for_wide_characters() {
        let view = prompt_view("ab\u{4e2d}cd", 3, 8);
        assert_eq!(view.text, "ab\u{4e2d}cd");
        assert_eq!(view.cursor, 4);
    }
}
