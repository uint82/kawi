use crossterm::style::Color;

use crate::frame::DiagnosticUnderline;
use crate::surface::Surface;
use crate::theme::{MatchColors, Theme};

pub fn apply_base(surface: &mut Surface, theme: &Theme) {
    apply_colors(surface, theme.base.foreground, theme.base.background);
}

pub fn apply_match(surface: &mut Surface, colors: &MatchColors) {
    apply_colors(surface, colors.foreground, colors.background);
}

pub fn apply_underline(surface: &mut Surface, color: Color, style: DiagnosticUnderline) {
    surface.set_underline(style, color);
}

pub fn apply_faded_text(surface: &mut Surface, color: Color) {
    surface.set_foreground(color);
    surface.set_dim(true);
}

pub fn apply_struck_text(surface: &mut Surface, color: Color) {
    surface.set_foreground(color);
    surface.set_crossed_out(true);
}

pub fn clear_decorations(surface: &mut Surface) {
    surface.reset_attributes();
}

pub fn restore_base(surface: &mut Surface, theme: &Theme) {
    apply_colors(surface, theme.base.foreground, theme.base.background);
}

pub fn apply_colors(surface: &mut Surface, foreground: Color, background: Color) {
    surface.set_colors(foreground, background);
}
