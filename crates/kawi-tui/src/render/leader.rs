use anyhow::{Context, Result};
use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Attribute, Color, SetAttribute},
};
use std::io::{Stdout, Write};

use crate::frame::{LeaderColumn, LeaderMenu, LeaderRow};
use crate::overlay::PANEL_BORDER;
use crate::paint;
use crate::text::{char_width, display_width, truncate_to_display_width};
use crate::theme::Theme;

use super::{clear_overlay_rect, draw_rounded_box, saturate_u16};

pub(super) fn draw(
    out: &mut Stdout,
    frame: &crate::frame::Frame,
    menu: &LeaderMenu<'_>,
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
    menu: &'a LeaderMenu<'a>,
    width: usize,
    height: usize,
}

impl LeaderSurface<'_> {
    fn draw(&self, out: &mut Stdout) -> Result<()> {
        let title = TitleRenderer {
            theme: self.theme,
            menu: self.menu,
            width: self.width,
        };
        title.draw(out)?;
        let columns = ColumnsRenderer {
            theme: self.theme,
            menu: self.menu,
            width: self.width,
            height: self.height.saturating_sub(1),
        };
        columns.draw(out)
    }
}

struct TitleRenderer<'a> {
    theme: &'a Theme,
    menu: &'a LeaderMenu<'a>,
    width: usize,
}

impl TitleRenderer<'_> {
    fn draw(&self, out: &mut Stdout) -> Result<()> {
        self.move_to(out)?;
        self.apply_style(out)?;
        let used = self.write_title(out)?;
        execute!(out, SetAttribute(Attribute::Reset)).context("reset leader title")?;
        self.apply_style(out)?;
        write_spaces(out, self.width.saturating_sub(used))?;
        paint::restore_base(out, self.theme)
    }

    fn move_to(&self, out: &mut Stdout) -> Result<()> {
        let x = self.menu.rect.x.saturating_add(PANEL_BORDER / 2);
        let y = self.menu.rect.y.saturating_add(PANEL_BORDER / 2);
        execute!(out, MoveTo(saturate_u16(x), saturate_u16(y))).context("move to leader title")
    }

    fn apply_style(&self, out: &mut Stdout) -> Result<()> {
        paint::apply_colors(
            out,
            self.theme.base.foreground,
            self.theme.float_background,
            "apply leader title",
        )?;
        execute!(out, SetAttribute(Attribute::Bold)).context("apply leader title style")
    }

    fn write_title(&self, out: &mut Stdout) -> Result<usize> {
        let title = truncate_to_display_width(self.menu.title, self.width);
        write!(out, "{title}").context("write leader title")?;
        let used = display_width(title);
        let Some(group) = self.menu.group else {
            return Ok(used);
        };
        let group_width = char_width(group);
        if used.saturating_add(1).saturating_add(group_width) > self.width {
            return Ok(used);
        }
        write!(out, " ").context("write leader title gap")?;
        paint::apply_colors(
            out,
            leader_key_color(self.theme),
            self.theme.float_background,
            "apply leader breadcrumb key",
        )?;
        write!(out, "{group}").context("write leader breadcrumb key")?;
        Ok(used.saturating_add(1).saturating_add(group_width))
    }
}

struct ColumnsRenderer<'a> {
    theme: &'a Theme,
    menu: &'a LeaderMenu<'a>,
    width: usize,
    height: usize,
}

impl ColumnsRenderer<'_> {
    fn draw(&self, out: &mut Stdout) -> Result<()> {
        for column in self.menu.columns {
            self.draw_column(out, column)?;
        }
        Ok(())
    }

    fn draw_column(&self, out: &mut Stdout, column: &LeaderColumn) -> Result<()> {
        let remaining = self.width.saturating_sub(column.offset);
        if remaining == 0 {
            return Ok(());
        }
        let renderer = ColumnRenderer {
            theme: self.theme,
            menu: self.menu,
            column,
            width: column.width.min(remaining),
        };
        renderer.draw(out, self.height)
    }
}

struct ColumnRenderer<'a> {
    theme: &'a Theme,
    menu: &'a LeaderMenu<'a>,
    column: &'a LeaderColumn,
    width: usize,
}

impl ColumnRenderer<'_> {
    fn draw(&self, out: &mut Stdout, height: usize) -> Result<()> {
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
            };
            renderer.draw(out, *row, offset)?;
        }
        Ok(())
    }
}

struct RowRenderer<'a> {
    theme: &'a Theme,
    menu: &'a LeaderMenu<'a>,
    column: &'a LeaderColumn,
    width: usize,
}

impl RowRenderer<'_> {
    fn draw(&self, out: &mut Stdout, row: LeaderRow<'_>, offset: usize) -> Result<()> {
        self.move_to(out, offset)?;
        self.write_key(out, row)?;
        self.write_description(out, row.description)?;
        paint::restore_base(out, self.theme)
    }

    fn move_to(&self, out: &mut Stdout, offset: usize) -> Result<()> {
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
            .saturating_add(offset);
        execute!(out, MoveTo(saturate_u16(x), saturate_u16(y))).context("move to leader row")
    }

    fn write_key(&self, out: &mut Stdout, row: LeaderRow<'_>) -> Result<()> {
        paint::apply_colors(
            out,
            leader_key_color(self.theme),
            self.theme.float_background,
            "apply leader key",
        )?;
        execute!(out, SetAttribute(Attribute::Bold)).context("apply leader key style")?;
        let label = row.key_label();
        let label = truncate_to_display_width(&label, self.width);
        write!(out, "{label}").context("write leader key")?;
        let description_offset = self.column.description_offset.min(self.width);
        write_spaces(out, description_offset.saturating_sub(display_width(label)))?;
        execute!(out, SetAttribute(Attribute::Reset)).context("reset leader key")
    }

    fn write_description(&self, out: &mut Stdout, description: &str) -> Result<()> {
        paint::apply_colors(
            out,
            self.theme.base.foreground,
            self.theme.float_background,
            "apply leader description",
        )?;
        let description_offset = self.column.description_offset.min(self.width);
        let width = self.width.saturating_sub(description_offset);
        let text = truncate_to_display_width(description, width);
        write!(out, "{text}").context("write leader description")?;
        let used = description_offset.saturating_add(display_width(text));
        write_spaces(out, self.width.saturating_sub(used))
    }
}

fn write_spaces(out: &mut Stdout, width: usize) -> Result<()> {
    write!(out, "{}", " ".repeat(width)).context("write leader padding")?;
    Ok(())
}

fn leader_key_color(theme: &Theme) -> Color {
    theme
        .syntax
        .named("keyword")
        .and_then(|style| style.foreground)
        .unwrap_or(theme.base.foreground)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_color_uses_the_theme_keyword() {
        let theme = Theme::default();
        let expected = theme
            .syntax
            .named("keyword")
            .and_then(|style| style.foreground)
            .expect("keyword color");
        assert_eq!(leader_key_color(&theme), expected);
        assert_ne!(leader_key_color(&theme), theme.base.foreground);
    }
}
