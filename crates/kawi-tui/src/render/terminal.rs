use anyhow::{Context, Result};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute,
    style::{Attribute, Color, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io::{Stdout, Write};

use super::{apply_cursor_style, paint, saturate_u16, write_padded};
use crate::frame::{TerminalAttrs, TerminalColor, TerminalPanel};
use crate::theme::Theme;

pub(super) fn draw_terminal_panel(out: &mut Stdout, frame: &crate::frame::Frame) -> Result<()> {
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
    out: &mut Stdout,
    frame: &crate::frame::Frame,
    panel: &TerminalPanel<'_>,
) -> Result<()> {
    clear_panel_line(out, panel.rect.x, panel.rect.y)?;
    execute!(out, SetAttribute(Attribute::Reset)).context("reset terminal header")?;
    let colors = if panel.focused {
        frame.theme.status.clone()
    } else {
        crate::theme::MatchColors {
            foreground: frame.theme.non_text,
            background: frame.theme.base.background,
        }
    };
    paint::apply_match(out, &colors)?;
    let header = format!("Terminal  {}  {}", panel.title, panel.status);
    write_padded(out, &header, panel.rect.width)?;
    execute!(out, SetAttribute(Attribute::Reset)).context("reset terminal header")?;
    paint::restore_base(out, frame.theme)?;
    Ok(())
}

fn draw_terminal_lines(
    out: &mut Stdout,
    frame: &crate::frame::Frame,
    panel: &TerminalPanel<'_>,
) -> Result<()> {
    for offset in 0..panel.rect.height.saturating_sub(1) {
        let row = panel.rect.y.saturating_add(offset).saturating_add(1);
        clear_panel_line(out, panel.rect.x, row)?;
        let Some(line) = panel.lines.get(offset) else {
            continue;
        };
        for span in &line.spans {
            apply_terminal_span(out, frame.theme, span)?;
        }
        execute!(out, SetAttribute(Attribute::Reset)).context("reset terminal attributes")?;
        paint::restore_base(out, frame.theme)?;
    }
    Ok(())
}

fn clear_panel_line(out: &mut impl Write, x: usize, y: usize) -> Result<()> {
    execute!(
        out,
        MoveTo(saturate_u16(x), saturate_u16(y)),
        Clear(ClearType::UntilNewLine)
    )
    .context("clear terminal line")
}

fn apply_terminal_span(
    out: &mut Stdout,
    theme: &Theme,
    span: &crate::frame::TerminalSpan,
) -> Result<()> {
    let (foreground, background) = terminal_colors(theme, span);
    execute!(
        out,
        SetAttribute(Attribute::Reset),
        SetForegroundColor(foreground),
        SetBackgroundColor(background)
    )
    .context("apply terminal span colors")?;
    apply_terminal_attrs(out, span.attrs)?;
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

fn apply_terminal_attrs(out: &mut Stdout, attrs: TerminalAttrs) -> Result<()> {
    if attrs.bold {
        execute!(out, SetAttribute(Attribute::Bold)).context("apply terminal bold")?;
    }
    if attrs.dim {
        execute!(out, SetAttribute(Attribute::Dim)).context("apply terminal dim")?;
    }
    if attrs.italic {
        execute!(out, SetAttribute(Attribute::Italic)).context("apply terminal italic")?;
    }
    if attrs.underline {
        execute!(out, SetAttribute(Attribute::Underlined)).context("apply terminal underline")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::clear_panel_line;

    #[test]
    fn clears_only_from_the_panel_start() {
        let mut out = Vec::new();
        clear_panel_line(&mut out, 67, 3).unwrap();
        assert_eq!(out, b"\x1b[4;68H\x1b[K");
    }
}
