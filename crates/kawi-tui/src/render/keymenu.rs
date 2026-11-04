use anyhow::{Context, Result};
use crossterm::style::Color;
use std::io::Write;

use crate::frame::{KeyColumn, KeyMenu, KeyRow};
use crate::overlay::PANEL_BORDER;
use crate::paint;
use crate::surface::Surface;
use crate::text::{display_width, truncate_to_display_width};
use crate::theme::Theme;

use super::overlay::{clear_overlay_rect, draw_rounded_box};

pub(super) fn draw(
    out: &mut Surface,
    frame: &crate::frame::Frame,
    menu: &KeyMenu<'_>,
) -> Result<()> {
    clear_overlay_rect(out, menu.rect, frame.theme)?;
    draw_rounded_box(out, menu.rect, frame.theme)?;
    let width = menu.rect.width.saturating_sub(PANEL_BORDER);
    let height = menu.rect.height.saturating_sub(PANEL_BORDER);
    if width == 0 || height == 0 {
        return Ok(());
    }
    let surface = LeaderSurface {
        theme: frame.theme,
        menu,
        width,
        height,
    };
    surface.draw(out)
}

struct LeaderSurface<'a> {
    theme: &'a Theme,
    menu: &'a KeyMenu<'a>,
    width: usize,
    height: usize,
}

impl LeaderSurface<'_> {
    fn draw(&self, out: &mut Surface) -> Result<()> {
        let title = TitleRenderer {
            theme: self.theme,
            menu: self.menu,
            width: self.width,
        };
        title.draw(out)?;
        let hint_rows = self.draw_hint(out)?;
        let columns = ColumnsRenderer {
            theme: self.theme,
            menu: self.menu,
            width: self.width,
            height: self.height.saturating_sub(1).saturating_sub(hint_rows),
            top: hint_rows,
        };
        columns.draw(out)
    }

    fn draw_hint(&self, out: &mut Surface) -> Result<usize> {
        let Some(hint) = self.menu.hint else {
            return Ok(0);
        };
        let renderer = HintRenderer {
            theme: self.theme,
            menu: self.menu,
            width: self.width,
        };
        renderer.draw(out, hint)?;
        Ok(1)
    }
}

struct TitleRenderer<'a> {
    theme: &'a Theme,
    menu: &'a KeyMenu<'a>,
    width: usize,
}

impl TitleRenderer<'_> {
    fn draw(&self, out: &mut Surface) -> Result<()> {
        self.move_to(out)?;
        self.apply_style(out)?;
        let used = self.write_title(out)?;
        out.reset_attributes();
        self.apply_style(out)?;
        write_spaces(out, self.width.saturating_sub(used))?;
        paint::restore_base(out, self.theme);
        Ok(())
    }

    fn move_to(&self, out: &mut Surface) -> Result<()> {
        let x = self.menu.rect.x.saturating_add(PANEL_BORDER / 2);
        let y = self.menu.rect.y.saturating_add(PANEL_BORDER / 2);
        out.move_to(x, y);
        Ok(())
    }

    fn apply_style(&self, out: &mut Surface) -> Result<()> {
        paint::apply_colors(out, self.theme.base.foreground, self.theme.float_background);
        out.set_bold(true);
        Ok(())
    }

    fn write_title(&self, out: &mut Surface) -> Result<usize> {
        let title = truncate_to_display_width(self.menu.title, self.width);
        write!(out, "{title}").context("write key menu title")?;
        let used = display_width(title);
        if self.menu.breadcrumb.is_empty() {
            return Ok(used);
        }
        let breadcrumb = truncate_to_display_width(
            self.menu.breadcrumb,
            self.width.saturating_sub(used.saturating_add(1)),
        );
        let breadcrumb_width = display_width(breadcrumb);
        if breadcrumb_width == 0 {
            return Ok(used);
        }
        write!(out, " ").context("write key menu title gap")?;
        paint::apply_colors(out, key_color(self.theme), self.theme.float_background);
        write!(out, "{breadcrumb}").context("write key menu breadcrumb")?;
        Ok(used.saturating_add(1).saturating_add(breadcrumb_width))
    }
}

struct ColumnsRenderer<'a> {
    theme: &'a Theme,
    menu: &'a KeyMenu<'a>,
    width: usize,
    height: usize,
    top: usize,
}

impl ColumnsRenderer<'_> {
    fn draw(&self, out: &mut Surface) -> Result<()> {
        for column in self.menu.columns {
            self.draw_column(out, column)?;
        }
        Ok(())
    }

    fn draw_column(&self, out: &mut Surface, column: &KeyColumn) -> Result<()> {
        let remaining = self.width.saturating_sub(column.offset);
        if remaining == 0 {
            return Ok(());
        }
        let renderer = ColumnRenderer {
            theme: self.theme,
            menu: self.menu,
            column,
            width: column.width.min(remaining),
            top: self.top,
        };
        renderer.draw(out, self.height)
    }
}

struct ColumnRenderer<'a> {
    theme: &'a Theme,
    menu: &'a KeyMenu<'a>,
    column: &'a KeyColumn,
    width: usize,
    top: usize,
}

impl ColumnRenderer<'_> {
    fn draw(&self, out: &mut Surface, height: usize) -> Result<()> {
        let rows = self.column.len.min(height);
        for offset in 0..rows {
            let Some(row) = self.menu.rows.get(self.column.start.saturating_add(offset)) else {
                break;
            };
            let renderer = RowRenderer {
                theme: self.theme,
                menu: self.menu,
                column: self.column,
                width: self.width,
                top: self.top,
            };
            renderer.draw(out, *row, offset)?;
        }
        Ok(())
    }
}

struct RowRenderer<'a> {
    theme: &'a Theme,
    menu: &'a KeyMenu<'a>,
    column: &'a KeyColumn,
    width: usize,
    top: usize,
}

impl RowRenderer<'_> {
    fn draw(&self, out: &mut Surface, row: KeyRow<'_>, offset: usize) -> Result<()> {
        self.move_to(out, offset)?;
        self.write_key(out, row)?;
        self.write_description(out, row.description)?;
        paint::restore_base(out, self.theme);
        Ok(())
    }

    fn move_to(&self, out: &mut Surface, offset: usize) -> Result<()> {
        let x = self
            .menu
            .rect
            .x
            .saturating_add(PANEL_BORDER / 2)
            .saturating_add(self.column.offset);
        let y = self
            .menu
            .rect
            .y
            .saturating_add(PANEL_BORDER / 2)
            .saturating_add(1)
            .saturating_add(self.top)
            .saturating_add(offset);
        out.move_to(x, y);
        Ok(())
    }

    fn write_key(&self, out: &mut Surface, row: KeyRow<'_>) -> Result<()> {
        paint::apply_colors(out, key_color(self.theme), self.theme.float_background);
        out.set_bold(true);
        let label = row.key_label();
        let label = truncate_to_display_width(&label, self.width);
        write!(out, "{label}").context("write key menu key")?;
        let description_offset = self.column.description_offset.min(self.width);
        write_spaces(out, description_offset.saturating_sub(display_width(label)))?;
        out.reset_attributes();
        Ok(())
    }

    fn write_description(&self, out: &mut Surface, description: &str) -> Result<()> {
        paint::apply_colors(out, self.theme.base.foreground, self.theme.float_background);
        let description_offset = self.column.description_offset.min(self.width);
        let width = self.width.saturating_sub(description_offset);
        let text = truncate_to_display_width(description, width);
        write!(out, "{text}").context("write key menu description")?;
        let used = description_offset.saturating_add(display_width(text));
        write_spaces(out, self.width.saturating_sub(used))
    }
}

struct HintRenderer<'a> {
    theme: &'a Theme,
    menu: &'a KeyMenu<'a>,
    width: usize,
}

impl HintRenderer<'_> {
    fn draw(&self, out: &mut Surface, hint: &str) -> Result<()> {
        self.move_to(out)?;
        self.apply_style(out)?;
        let text = truncate_to_display_width(hint, self.width);
        write!(out, "{text}").context("write key menu hint")?;
        write_spaces(out, self.width.saturating_sub(display_width(text)))?;
        paint::restore_base(out, self.theme);
        Ok(())
    }

    fn move_to(&self, out: &mut Surface) -> Result<()> {
        let x = self.menu.rect.x.saturating_add(PANEL_BORDER / 2);
        let y = self
            .menu
            .rect
            .y
            .saturating_add(PANEL_BORDER / 2)
            .saturating_add(1);
        out.move_to(x, y);
        Ok(())
    }

    fn apply_style(&self, out: &mut Surface) -> Result<()> {
        paint::apply_colors(out, self.theme.base.foreground, self.theme.float_background);
        Ok(())
    }
}

fn write_spaces(out: &mut Surface, width: usize) -> Result<()> {
    write!(out, "{}", " ".repeat(width)).context("write key menu padding")?;
    Ok(())
}

fn key_color(theme: &Theme) -> Color {
    theme
        .syntax
        .named("keyword")
        .and_then(|style| style.foreground)
        .unwrap_or(theme.base.foreground)
}

#[cfg(test)]
mod tests;
