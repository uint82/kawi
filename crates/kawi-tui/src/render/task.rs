use anyhow::{Context, Result};
use std::io::Write;

use crate::frame::{Frame, TaskPanel};
use crate::paint;
use crate::surface::Surface;
use crate::text::truncate_to_display_width;

use super::furniture::write_padded;

pub(super) fn draw_task_panel(out: &mut Surface, frame: &Frame) -> Result<()> {
    let Some(panel) = frame.task.as_ref() else {
        return Ok(());
    };
    draw_task_header(out, frame, panel)?;
    draw_task_lines(out, frame, panel)?;
    Ok(())
}

fn draw_task_header(out: &mut Surface, frame: &Frame, panel: &TaskPanel<'_>) -> Result<()> {
    out.move_to(panel.rect.x, panel.rect.y);
    out.clear_line();
    paint::apply_match(out, &frame.theme.status);
    let header = format!("{}  {}", panel.title, panel.status);
    write_padded(out, &header, panel.rect.width)?;
    paint::restore_base(out, frame.theme);
    Ok(())
}

fn draw_task_lines(out: &mut Surface, frame: &Frame, panel: &TaskPanel<'_>) -> Result<()> {
    for offset in 0..panel.rect.height.saturating_sub(1) {
        let row = panel.rect.y.saturating_add(offset).saturating_add(1);
        out.move_to(panel.rect.x, row);
        out.clear_line();
        let Some(line) = panel.lines.get(offset) else {
            continue;
        };
        if line.stderr {
            out.set_foreground(frame.theme.diagnostic_warning);
        }
        write!(
            out,
            "{}",
            truncate_to_display_width(line.text, panel.rect.width)
        )
        .context("write task line")?;
        paint::restore_base(out, frame.theme);
    }
    Ok(())
}
