use std::io::Write;

use anyhow::{Context, Result};
use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Attribute, SetAttribute, SetBackgroundColor, SetForegroundColor, SetUnderlineColor},
};

use crate::frame::DiagnosticUnderline;

use super::{Style, Surface};

pub fn present<W: Write>(surface: &Surface, out: &mut W) -> Result<()> {
    for y in 0..surface.height() {
        present_row(surface, y, out)?;
    }
    Ok(())
}

fn present_row<W: Write>(surface: &Surface, y: usize, out: &mut W) -> Result<()> {
    execute!(out, MoveTo(0, saturate_u16(y))).context("move to surface row")?;
    let mut current = None;
    for x in 0..surface.width() {
        let Some(cell) = surface.cell(x, y) else {
            continue;
        };
        if cell.is_continuation() {
            continue;
        }
        if current != Some(cell.style()) {
            apply_style(out, cell.style())?;
            current = Some(cell.style());
        }
        write!(out, "{}", cell.text()).context("write surface cell")?;
    }
    Ok(())
}

fn apply_style<W: Write>(out: &mut W, style: Style) -> Result<()> {
    execute!(
        out,
        SetAttribute(Attribute::Reset),
        SetForegroundColor(style.foreground()),
        SetBackgroundColor(style.background())
    )
    .context("apply surface colors")?;
    if style.is_bold() {
        execute!(out, SetAttribute(Attribute::Bold)).context("apply surface bold")?;
    }
    if style.is_dim() {
        execute!(out, SetAttribute(Attribute::Dim)).context("apply surface dim")?;
    }
    if style.is_italic() {
        execute!(out, SetAttribute(Attribute::Italic)).context("apply surface italic")?;
    }
    match style.underline() {
        Some(DiagnosticUnderline::Straight) => {
            execute!(out, SetAttribute(Attribute::Underlined))
                .context("apply surface underline")?;
        }
        Some(DiagnosticUnderline::Curly) => {
            execute!(out, SetAttribute(Attribute::Undercurled))
                .context("apply surface undercurl")?;
        }
        None => {}
    }
    if let Some(color) = style.underline_color() {
        execute!(out, SetUnderlineColor(color)).context("apply surface underline color")?;
    }
    if style.is_crossed_out() {
        execute!(out, SetAttribute(Attribute::CrossedOut))
            .context("apply surface strikethrough")?;
    }
    if style.is_hidden() {
        execute!(out, SetAttribute(Attribute::Hidden)).context("apply surface hidden")?;
    }
    Ok(())
}

fn saturate_u16(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests {
    use crossterm::style::Color;

    use super::*;

    #[test]
    fn presents_wide_graphemes_once() {
        let mut surface = Surface::new(4, 1, Color::White, Color::Black);
        surface.write_text("a中");
        let mut out = Vec::new();
        present(&surface, &mut out).expect("present surface");
        let output = String::from_utf8(out).expect("utf8");
        assert_eq!(output.matches('中').count(), 1);
        assert!(output.contains("a中"));
    }
}
