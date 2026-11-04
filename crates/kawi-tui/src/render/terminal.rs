use anyhow::{Context, Result};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute,
    style::Color,
};
use std::io::{Stdout, Write};

use super::cursor::apply_cursor_style;
use super::furniture::{saturate_u16, write_padded};
use super::paint;
use crate::frame::{TerminalAttrs, TerminalColor, TerminalPanel};
use crate::surface::Surface;
use crate::theme::Theme;

pub(super) fn draw_terminal_panel(out: &mut Surface, frame: &crate::frame::Frame) -> Result<()> {
    let Some(panel) = frame.terminal.as_ref() else {
        return Ok(());
    };
    draw_terminal_header(out, frame, panel)?;
    draw_terminal_lines(out, frame, panel)?;
    Ok(())
}

pub(super) fn place_terminal_cursor(out: &mut Stdout, panel: &TerminalPanel<'_>) -> Result<()> {
    let Some(cursor) = panel.cursor else {
        execute!(out, Hide).context("hide terminal cursor")?;
        return Ok(());
    };
    let x = panel
        .rect
        .x
        .saturating_add(cursor.col.min(panel.rect.width.saturating_sub(1)));
    let y = panel
        .rect
        .y
        .saturating_add(cursor.row.min(panel.rect.height.saturating_sub(2)))
        .saturating_add(1);
    execute!(out, Show, MoveTo(saturate_u16(x), saturate_u16(y)))
        .context("place terminal cursor")?;
    apply_cursor_style(out, cursor.style)
}

fn draw_terminal_header(
    out: &mut Surface,
    frame: &crate::frame::Frame,
    panel: &TerminalPanel<'_>,
) -> Result<()> {
    clear_panel_line(out, panel.rect.x, panel.rect.y);
    out.reset_attributes();
    let colors = if panel.focused {
        frame.theme.status.clone()
    } else {
        crate::theme::MatchColors {
            foreground: frame.theme.non_text,
            background: frame.theme.base.background,
        }
    };
    paint::apply_match(out, &colors);
    let header = format!("Terminal  {}  {}", panel.title, panel.status);
    write_padded(out, &header, panel.rect.width)?;
    out.reset_attributes();
    paint::restore_base(out, frame.theme);
    Ok(())
}

fn draw_terminal_lines(
    out: &mut Surface,
    frame: &crate::frame::Frame,
    panel: &TerminalPanel<'_>,
) -> Result<()> {
    for offset in 0..panel.rect.height.saturating_sub(1) {
        let row = panel.rect.y.saturating_add(offset).saturating_add(1);
        clear_panel_line(out, panel.rect.x, row);
        let Some(line) = panel.lines.get(offset) else {
            continue;
        };
        for span in &line.spans {
            apply_terminal_span(out, frame.theme, span)?;
        }
        out.reset_attributes();
        paint::restore_base(out, frame.theme);
    }
    Ok(())
}

fn clear_panel_line(out: &mut Surface, x: usize, y: usize) {
    out.move_to(x, y);
    out.clear_tail();
}

fn apply_terminal_span(
    out: &mut Surface,
    theme: &Theme,
    span: &crate::frame::TerminalSpan,
) -> Result<()> {
    let (foreground, background) = terminal_colors(theme, span);
    out.reset_attributes();
    out.set_colors(foreground, background);
    apply_terminal_attrs(out, span.attrs);
    write!(out, "{}", span.text).context("write terminal span")?;
    Ok(())
}

fn terminal_colors(theme: &Theme, span: &crate::frame::TerminalSpan) -> (Color, Color) {
    let foreground = terminal_color(span.foreground, theme.base.foreground);
    let background = terminal_color(span.background, theme.base.background);
    if span.attrs.inverse {
        return (background, foreground);
    }
    (foreground, background)
}

fn terminal_color(color: TerminalColor, default: Color) -> Color {
    match color {
        TerminalColor::Default => default,
        TerminalColor::Indexed(index) => Color::AnsiValue(index),
        TerminalColor::Rgb(red, green, blue) => Color::Rgb {
            r: red,
            g: green,
            b: blue,
        },
    }
}

fn apply_terminal_attrs(out: &mut Surface, attrs: TerminalAttrs) {
    out.set_bold(attrs.bold);
    out.set_dim(attrs.dim);
    out.set_italic(attrs.italic);
    if attrs.underline {
        out.set_plain_underline();
    }
}

#[cfg(test)]
mod tests {
    use crossterm::style::Color;

    use super::*;

    #[test]
    fn clears_only_from_the_panel_start() {
        let mut out = Surface::new(5, 4, Color::White, Color::Black);
        out.move_to(0, 3);
        out.write_text("abcde");
        clear_panel_line(&mut out, 2, 3);
        assert_eq!(out.cell(1, 3).expect("kept cell").text(), "b");
        assert_eq!(out.cell(2, 3).expect("cleared cell").text(), " ");
        assert_eq!(out.cell(4, 3).expect("cleared cell").text(), " ");
    }
}
