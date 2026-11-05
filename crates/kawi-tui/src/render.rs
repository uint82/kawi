use anyhow::{Context, Result};
use crossterm::{
    cursor::Show,
    execute,
    style::Color,
    terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate},
};
use std::io::{Stdout, Write};

use crate::frame::{CursorStyle, Frame, PaneFrame};
use crate::paint;
use crate::statusbar::{draw_status, StatusArea};
use crate::surface::{self, Surface};

mod bracket;
mod cursor;
mod debug;
mod diagnostic;
mod explorer;
mod furniture;
mod highlight;
mod keymenu;
mod line;
mod menu;
mod message;
mod overlay;
mod pager;
mod picker;
mod plugin_panel;
mod replace;
mod selection;
mod special_key;
mod style;
mod task;
mod terminal;
mod virtual_text;

use cursor::{apply_cursor_style, place_cursor};
use furniture::draw_filler_rows;
use highlight::draw_highlight;
use message::draw_message_row;
use overlay::draw_overlay;
use pager::draw_pager;
use replace::{draw_replace, place_replace_cursor};
use task::draw_task_panel;

use debug::draw_debug_panel;
use explorer::draw_explorer_panel;
use line::draw_visible_lines;
use plugin_panel::draw_plugin_panel;
use terminal::{draw_terminal_panel, place_terminal_cursor};

pub struct Renderer {
    surface: Surface,
    previous: Surface,
    buffer: Vec<u8>,
    has_frame: bool,
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            surface: Surface::new(1, 1, Color::Reset, Color::Reset),
            previous: Surface::new(1, 1, Color::Reset, Color::Reset),
            buffer: Vec::new(),
            has_frame: false,
        }
    }

    pub fn draw(&mut self, out: &mut Stdout, frame: &Frame) -> Result<()> {
        execute!(out, BeginSynchronizedUpdate).context("begin synchronized update")?;
        let contents = self.paint(out, frame);
        let finished = execute!(out, EndSynchronizedUpdate).context("end synchronized update");
        contents.and(finished)
    }

    fn paint(&mut self, out: &mut Stdout, frame: &Frame) -> Result<()> {
        let is_resized = self.render(frame)?;
        self.present(out, is_resized)?;
        place_frame_cursor(out, frame)?;
        flush(out)
    }

    fn render(&mut self, frame: &Frame) -> Result<bool> {
        let width = frame.viewport.width();
        let height = frame.viewport.height().saturating_add(2);
        let is_resized = self.previous.width() != width || self.previous.height() != height;
        self.surface.fit(
            width,
            height,
            frame.theme.base.foreground,
            frame.theme.base.background,
        );
        match (frame.replace.as_ref(), frame.picker.as_ref()) {
            (Some(replace), _) => draw_replace(&mut self.surface, frame, replace),
            (None, Some(picker)) => picker::draw(&mut self.surface, frame, picker),
            (None, None) => draw_surface(&mut self.surface, frame),
        }?;
        Ok(is_resized)
    }

    fn present(&mut self, out: &mut Stdout, is_resized: bool) -> Result<()> {
        self.buffer.clear();
        if self.has_frame && !is_resized {
            surface::diff(&self.previous, &self.surface, &mut self.buffer)?;
        } else {
            surface::present(&self.surface, &mut self.buffer)?;
        }
        out.write_all(&self.buffer).context("write surface")?;
        std::mem::swap(&mut self.previous, &mut self.surface);
        self.has_frame = true;
        Ok(())
    }
}

fn place_frame_cursor(out: &mut Stdout, frame: &Frame) -> Result<()> {
    if let Some(replace) = frame.replace.as_ref() {
        place_replace_cursor(out, frame, replace)?;
        return apply_cursor_style(out, CursorStyle::Bar);
    }
    if let Some(picker) = frame.picker.as_ref() {
        apply_cursor_style(out, CursorStyle::Bar)?;
        return picker::place_cursor(out, frame, picker);
    }
    if frame
        .explorer
        .as_ref()
        .is_some_and(|explorer| explorer.focused)
        || frame.debug.as_ref().is_some_and(|debug| debug.focused)
        || frame
            .plugin_panel
            .as_ref()
            .is_some_and(|panel| panel.focused)
    {
        execute!(out, crossterm::cursor::Hide).context("hide panel cursor")?;
        return Ok(());
    }
    if let Some(terminal) = frame.terminal.as_ref().filter(|terminal| terminal.focused) {
        return place_terminal_cursor(out, terminal);
    }
    execute!(out, Show).context("show editor cursor")?;
    apply_cursor_style(out, frame.cursor_style)?;
    place_cursor(out, frame)
}

fn draw_surface(out: &mut Surface, frame: &Frame) -> Result<()> {
    paint::apply_base(out, frame.theme);
    for pane in frame.panes {
        draw_pane(out, frame, pane)?;
    }
    draw_separators(out, frame)?;
    draw_explorer_panel(out, frame)?;
    draw_debug_panel(out, frame)?;
    draw_plugin_panel(out, frame)?;
    draw_task_panel(out, frame)?;
    draw_terminal_panel(out, frame)?;
    match frame.pager {
        Some(lines) => draw_pager(out, frame, lines)?,
        None => draw_message_row(out, frame)?,
    }
    menu::draw_completion(out, frame)?;
    draw_overlay(out, frame)?;
    Ok(())
}

fn draw_pane(out: &mut Surface, frame: &Frame, pane: &PaneFrame<'_>) -> Result<()> {
    draw_visible_lines(out, pane, frame.theme)?;
    draw_filler_rows(out, pane, frame.theme)?;
    draw_highlight(out, pane, frame.theme)?;
    if pane.rect.height == 0 {
        return Ok(());
    }
    let row = pane
        .rect
        .y
        .saturating_add(pane.rect.height.saturating_sub(1));
    draw_status(
        out,
        pane.status,
        StatusArea {
            x: pane.rect.x,
            y: row,
            width: pane.rect.width,
            active: pane.active,
        },
        frame.theme,
        frame.statusline,
    )
}

fn draw_separators(out: &mut Surface, frame: &Frame) -> Result<()> {
    if frame.separators.is_empty() {
        return Ok(());
    }
    out.set_foreground(frame.theme.non_text);
    for separator in frame.separators {
        for offset in 0..separator.height {
            out.move_to(separator.x, separator.y.saturating_add(offset));
            write!(out, "│").context("write separator")?;
        }
    }
    paint::restore_base(out, frame.theme);
    Ok(())
}

pub fn prompt_cursor_col(
    command: Option<&str>,
    search: Option<&str>,
    width: usize,
    showcmd: Option<&str>,
) -> Option<usize> {
    cursor::prompt_cursor_col(command, search, width, showcmd)
}

fn picker_start(available: usize, selected: Option<usize>) -> usize {
    match selected {
        Some(index) if index >= available => index.saturating_add(1).saturating_sub(available),
        _ => 0,
    }
}

fn flush(out: &mut Stdout) -> Result<()> {
    out.flush().context("flush stdout")?;
    Ok(())
}

#[cfg(test)]
mod tests;
