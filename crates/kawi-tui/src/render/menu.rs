use anyhow::Result;

use crate::frame::Frame;
use crate::overlay::COMPLETION_MAX_ROWS;
use crate::paint;
use crate::surface::Surface;

use super::furniture::write_padded;

pub(super) fn draw_completion(out: &mut Surface, frame: &Frame) -> Result<()> {
    let Some(menu) = frame.completion.as_ref() else {
        return Ok(());
    };
    if menu.items.is_empty() {
        return Ok(());
    }
    let Some(pane) = frame.panes.iter().find(|pane| pane.active) else {
        return Ok(());
    };
    let height = pane.rect.height.saturating_sub(1);
    let shown = menu.items.len().min(visible_rows(height));
    if shown == 0 {
        return Ok(());
    }
    let viewport = CompletionViewport {
        items: menu.items,
        selected: menu.selected,
        x: pane.rect.x,
        width: pane.rect.width,
        start: visible_start(shown, menu.selected),
        shown,
        top: pane.rect.y.saturating_add(height.saturating_sub(shown)),
    };
    draw_rows(out, frame, viewport)
}

struct CompletionViewport<'a> {
    items: &'a [String],
    selected: Option<usize>,
    x: usize,
    width: usize,
    start: usize,
    shown: usize,
    top: usize,
}

fn draw_rows(out: &mut Surface, frame: &Frame, viewport: CompletionViewport<'_>) -> Result<()> {
    for offset in 0..viewport.shown {
        let index = viewport.start + offset;
        draw_row(
            out,
            frame,
            CompletionRow {
                text: &viewport.items[index],
                selected: viewport.selected == Some(index),
                x: viewport.x,
                y: viewport.top.saturating_add(offset),
                width: viewport.width,
            },
        )?;
    }
    Ok(())
}

struct CompletionRow<'a> {
    text: &'a str,
    selected: bool,
    x: usize,
    y: usize,
    width: usize,
}

fn draw_row(out: &mut Surface, frame: &Frame, row: CompletionRow<'_>) -> Result<()> {
    out.move_to(row.x, row.y);
    if row.selected {
        paint::apply_match(out, &frame.theme.selection);
    }
    write_padded(out, row.text, row.width)?;
    paint::restore_base(out, frame.theme);
    Ok(())
}

fn visible_rows(height: usize) -> usize {
    height.min(COMPLETION_MAX_ROWS)
}

pub(super) fn visible_start(shown: usize, selected: Option<usize>) -> usize {
    match selected {
        Some(index) if index >= shown => index.saturating_add(1).saturating_sub(shown),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_rows_caps_the_menu() {
        assert_eq!(visible_rows(100), COMPLETION_MAX_ROWS);
        assert_eq!(visible_rows(4), 4);
    }

    #[test]
    fn visible_start_keeps_the_selection_in_view() {
        assert_eq!(visible_start(10, None), 0);
        assert_eq!(visible_start(10, Some(0)), 0);
        assert_eq!(visible_start(10, Some(9)), 0);
        assert_eq!(visible_start(10, Some(10)), 1);
        assert_eq!(visible_start(10, Some(25)), 16);
    }
}
