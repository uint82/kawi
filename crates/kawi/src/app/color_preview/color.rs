use crossterm::style::Color;
use kawi_tui::frame::PluginStyle;

use super::contrast;

const OPAQUE: u8 = 255;
const MAX_CHANNEL: u32 = 255;
const ROUNDING_BIAS: u32 = 127;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Rgba {
    pub(super) red: u8,
    pub(super) green: u8,
    pub(super) blue: u8,
    pub(super) alpha: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Token {
    pub(super) color: Rgba,
    pub(super) len: usize,
}

impl Rgba {
    pub(super) const fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    pub(super) fn to_style(self, theme_background: Color) -> PluginStyle {
        let (red, green, blue) = self.blend(theme_background);
        let background = Color::Rgb {
            r: red,
            g: green,
            b: blue,
        };
        PluginStyle {
            foreground: contrast::foreground(red, green, blue),
            background,
            accent: background,
            bold: false,
            italic: false,
            underline: false,
            hidden: false,
        }
    }

    fn blend(self, theme_background: Color) -> (u8, u8, u8) {
        if self.alpha == OPAQUE {
            return (self.red, self.green, self.blue);
        }
        let Some((red, green, blue)) = background_rgb(theme_background) else {
            return (self.red, self.green, self.blue);
        };
        (
            blend_channel(self.red, red, self.alpha),
            blend_channel(self.green, green, self.alpha),
            blend_channel(self.blue, blue, self.alpha),
        )
    }
}

impl Token {
    pub(super) const fn new(color: Rgba, len: usize) -> Self {
        Self { color, len }
    }
}

fn blend_channel(foreground: u8, background: u8, alpha: u8) -> u8 {
    let inverse = u32::from(OPAQUE - alpha);
    let value =
        u32::from(foreground) * u32::from(alpha) + u32::from(background) * inverse + ROUNDING_BIAS;
    u8::try_from(value / MAX_CHANNEL).unwrap_or(OPAQUE)
}

fn background_rgb(color: Color) -> Option<(u8, u8, u8)> {
    match color {
        Color::Reset => None,
        Color::Black => Some((0, 0, 0)),
        Color::DarkGrey => Some((85, 85, 85)),
        Color::Red => Some((170, 0, 0)),
        Color::DarkRed => Some((85, 0, 0)),
        Color::Green => Some((0, 170, 0)),
        Color::DarkGreen => Some((0, 85, 0)),
        Color::Yellow => Some((170, 85, 0)),
        Color::DarkYellow => Some((85, 85, 0)),
        Color::Blue => Some((0, 0, 170)),
        Color::DarkBlue => Some((0, 0, 85)),
        Color::Magenta => Some((170, 0, 170)),
        Color::DarkMagenta => Some((85, 0, 85)),
        Color::Cyan => Some((0, 170, 170)),
        Color::DarkCyan => Some((0, 85, 85)),
        Color::White => Some((255, 255, 255)),
        Color::Grey => Some((170, 170, 170)),
        Color::Rgb { r, g, b } => Some((r, g, b)),
        Color::AnsiValue(_) => None,
    }
}
