use anyhow::{Context, Result};
use crossterm::{
    execute,
    style::{
        Attribute, Color, SetAttribute, SetBackgroundColor, SetForegroundColor, SetUnderlineColor,
    },
};
use std::io::Write;

use crate::theme::{MatchColors, Theme};

pub fn apply_base<W: Write>(out: &mut W, theme: &Theme) -> Result<()> {
    apply_colors(
        out,
        theme.base.foreground,
        theme.base.background,
        "apply base colors",
    )
}

pub fn apply_match<W: Write>(out: &mut W, colors: &MatchColors) -> Result<()> {
    apply_colors(out, colors.foreground, colors.background, "tint span")
}

pub fn apply_underline<W: Write>(out: &mut W, color: Color) -> Result<()> {
    execute!(
        out,
        SetAttribute(Attribute::Underlined),
        SetUnderlineColor(color)
    )
    .context("apply diagnostic underline")?;
    Ok(())
}

pub fn apply_faded_text<W: Write>(out: &mut W, color: Color) -> Result<()> {
    execute!(out, SetForegroundColor(color), SetAttribute(Attribute::Dim))
        .context("apply faded diagnostic text")?;
    Ok(())
}

pub fn apply_struck_text<W: Write>(out: &mut W, color: Color) -> Result<()> {
    execute!(
        out,
        SetForegroundColor(color),
        SetAttribute(Attribute::CrossedOut)
    )
    .context("apply struck diagnostic text")?;
    Ok(())
}

pub fn clear_decorations<W: Write>(out: &mut W) -> Result<()> {
    execute!(out, SetAttribute(Attribute::Reset)).context("clear diagnostic decorations")?;
    Ok(())
}

pub fn restore_base<W: Write>(out: &mut W, theme: &Theme) -> Result<()> {
    apply_colors(
        out,
        theme.base.foreground,
        theme.base.background,
        "restore base colors",
    )
}

pub fn apply_colors<W: Write>(
    out: &mut W,
    foreground: Color,
    background: Color,
    context: &'static str,
) -> Result<()> {
    execute!(
        out,
        SetForegroundColor(foreground),
        SetBackgroundColor(background)
    )
    .context(context)?;
    Ok(())
}
