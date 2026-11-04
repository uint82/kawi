use std::io::Write;

use anyhow::{Context, Result};

use super::furniture::{draw_rulers, erase_row_tail, gutter_width};
use super::virtual_text::{draw_left_virtual_text, draw_virtual_text, VirtualTextDraw};
use crate::display;
use crate::frame::PaneFrame;
use crate::paint;
use crate::surface::Surface;
use crate::theme::Theme;

pub(super) fn draw_visible_lines(
    out: &mut Surface,
    pane: &PaneFrame<'_>,
    theme: &Theme,
) -> Result<()> {
    let gutter_width = gutter_width(pane);
    let height = pane.rect.height.saturating_sub(1);
    let mut line = LineDraw {
        out,
        pane,
        theme,
        gutter_width,
        scratch: String::new(),
    };
    for (screen_row, text) in pane.lines.iter().enumerate().take(height) {
        line.draw(screen_row, text)?;
    }
    Ok(())
}

struct LineDraw<'a, 'p> {
    out: &'a mut Surface,
    pane: &'a PaneFrame<'p>,
    theme: &'a Theme,
    gutter_width: usize,
    scratch: String,
}

impl LineDraw<'_, '_> {
    fn draw(&mut self, screen_row: usize, text: &str) -> Result<()> {
        self.draw_prefix(screen_row)?;
        let visible = self.draw_text(text)?;
        draw_rulers(
            self.out,
            self.pane,
            self.theme,
            screen_row,
            self.gutter_width,
            visible,
        )?;
        draw_virtual_text(
            self.out,
            VirtualTextDraw {
                pane: self.pane,
                theme: self.theme,
                screen_row,
                gutter_width: self.gutter_width,
                width: self.pane.rect.width.saturating_sub(self.gutter_width),
                line_width: visible,
            },
        )
    }

    fn draw_prefix(&mut self, screen_row: usize) -> Result<()> {
        self.out.move_to(
            self.pane.rect.x,
            self.pane.rect.y.saturating_add(screen_row),
        );
        self.draw_gutter(screen_row)?;
        draw_left_virtual_text(self.out, self.pane, self.theme, screen_row)
    }

    fn draw_gutter(&mut self, screen_row: usize) -> Result<()> {
        if let Some(signs) = self.pane.signs {
            match signs.get(screen_row).and_then(Option::as_ref) {
                Some(sign) => {
                    self.out.set_foreground(self.theme.sign_color(sign));
                    write!(self.out, "{}", sign.text()).context("write sign")?;
                }
                None => write!(self.out, " ").context("write empty sign")?,
            }
            self.out.set_foreground(self.theme.line_number);
        }
        if let Some(gutter) = self.pane.gutter.get(screen_row) {
            self.out.set_foreground(self.theme.line_number);
            write!(self.out, "{gutter}").context("write gutter")?;
        }
        if self.pane.signs.is_some() || self.pane.gutter.get(screen_row).is_some() {
            paint::restore_base(self.out, self.theme);
        }
        Ok(())
    }

    fn draw_text(&mut self, text: &str) -> Result<usize> {
        let width = self.pane.rect.width.saturating_sub(self.gutter_width);
        let end = self.pane.viewport.leftcol().saturating_add(width);
        let visible = display::slice_screen_into(
            text,
            self.pane.viewport.leftcol(),
            end,
            self.pane.tabstop,
            &mut self.scratch,
        );
        self.out
            .write_all(self.scratch.as_bytes())
            .context("write line")?;
        erase_row_tail(self.out)?;
        Ok(visible)
    }
}
