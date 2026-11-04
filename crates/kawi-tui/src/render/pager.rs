use anyhow::{Context, Result};
use std::io::Write;

use crate::frame::Frame;
use crate::surface::Surface;
use crate::text::sanitize_text;

use super::furniture::truncate_to_width;

pub(super) const PAGER_PROMPT: &str = "Press ENTER or type command to continue";

pub(super) fn draw_pager(out: &mut Surface, frame: &Frame, lines: &[String]) -> Result<()> {
    let width = frame.viewport.width();
    let available = frame.viewport.height().saturating_add(1);
    let visible = &lines[lines.len().saturating_sub(available)..];
    for (index, line) in visible.iter().enumerate() {
        let row = frame
            .viewport
            .height()
            .saturating_sub(visible.len().saturating_sub(1).saturating_sub(index));
        out.move_to(0, row);
        out.clear_line();
        let line = sanitize_text(line);
        write!(out, "{}", truncate_to_width(&line, width)).context("write pager line")?;
    }
    let prompt_row = frame.viewport.height().saturating_add(1);
    out.move_to(0, prompt_row);
    out.clear_line();
    write!(out, "{}", truncate_to_width(PAGER_PROMPT, width)).context("write pager prompt")?;
    Ok(())
}
