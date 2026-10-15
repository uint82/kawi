use anyhow::{Context, Result};
use crossterm::{
    execute,
    style::{Color, SetBackgroundColor, SetForegroundColor},
};
use std::io::Stdout;

use crate::theme::{MatchColors, Theme};

pub fn apply_base(out: &mut Stdout, theme: &Theme) -> Result<()> {
    apply_colors(
        out,
        theme.base.foreground,
        theme.base.background,
        "apply base colors",
    )
}

pub fn apply_match(out: &mut Stdout, colors: &MatchColors) -> Result<()> {
    apply_colors(out, colors.foreground, colors.background, "tint span")
}

pub fn restore_base(out: &mut Stdout, theme: &Theme) -> Result<()> {
    apply_colors(
        out,
        theme.base.foreground,
        theme.base.background,
        "restore base colors",
    )
}

fn apply_colors(
    out: &mut Stdout,
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
