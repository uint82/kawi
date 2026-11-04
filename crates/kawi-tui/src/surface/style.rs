use crossterm::style::Color;

use crate::frame::DiagnosticUnderline;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    foreground: Color,
    background: Color,
    underline_color: Option<Color>,
    underline: Option<DiagnosticUnderline>,
    bold: bool,
    dim: bool,
    italic: bool,
    crossed_out: bool,
    hidden: bool,
}

impl Style {
    pub fn new(foreground: Color, background: Color) -> Self {
        Self {
            foreground,
            background,
            underline_color: None,
            underline: None,
            bold: false,
            dim: false,
            italic: false,
            crossed_out: false,
            hidden: false,
        }
    }

    pub fn foreground(self) -> Color {
        self.foreground
    }

    pub fn background(self) -> Color {
        self.background
    }

    pub fn underline_color(self) -> Option<Color> {
        self.underline_color
    }

    pub fn underline(self) -> Option<DiagnosticUnderline> {
        self.underline
    }

    pub fn is_bold(self) -> bool {
        self.bold
    }

    pub fn is_dim(self) -> bool {
        self.dim
    }

    pub fn is_italic(self) -> bool {
        self.italic
    }

    pub fn is_crossed_out(self) -> bool {
        self.crossed_out
    }

    pub fn is_hidden(self) -> bool {
        self.hidden
    }

    pub fn set_foreground(&mut self, color: Color) {
        self.foreground = color;
    }

    pub fn set_background(&mut self, color: Color) {
        self.background = color;
    }

    pub fn set_colors(&mut self, foreground: Color, background: Color) {
        self.foreground = foreground;
        self.background = background;
    }

    pub fn set_bold(&mut self, enabled: bool) {
        self.bold = enabled;
    }

    pub fn set_dim(&mut self, enabled: bool) {
        self.dim = enabled;
    }

    pub fn set_italic(&mut self, enabled: bool) {
        self.italic = enabled;
    }

    pub fn set_crossed_out(&mut self, enabled: bool) {
        self.crossed_out = enabled;
    }

    pub fn set_hidden(&mut self, enabled: bool) {
        self.hidden = enabled;
    }

    pub fn set_underline(&mut self, style: DiagnosticUnderline, color: Color) {
        self.underline = Some(style);
        self.underline_color = Some(color);
    }

    pub fn set_plain_underline(&mut self) {
        self.underline = Some(DiagnosticUnderline::Straight);
        self.underline_color = None;
    }

    pub fn reset_attributes(&mut self) {
        self.underline_color = None;
        self.underline = None;
        self.bold = false;
        self.dim = false;
        self.italic = false;
        self.crossed_out = false;
        self.hidden = false;
    }
}
