use crossterm::style::Color;
use kawi_tui::frame::PluginStyle;

const OPAQUE: u8 = 255;
const MAX_CHANNEL: u32 = 255;
const ROUNDING_BIAS: u32 = 127;
const HEX_RADIX: u8 = 16;
const NIBBLE_SCALE: u8 = 17;
const LUMINANCE_RED: f64 = 0.2126;
const LUMINANCE_GREEN: f64 = 0.7152;
const LUMINANCE_BLUE: f64 = 0.0722;
const CONTRAST_OFFSET: f64 = 0.05;
const GAMMA_THRESHOLD: f64 = 0.03928;
const GAMMA_DIVISOR: f64 = 12.92;
const GAMMA_OFFSET: f64 = 0.055;
const GAMMA_SCALE: f64 = 1.055;
const GAMMA_EXPONENT: f64 = 2.4;
const BLACK: u8 = 0;
const WHITE: u8 = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HexColor {
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

impl HexColor {
    pub(super) fn from_digits(digits: &[u8]) -> Option<Self> {
        match digits.len() {
            3 => Self::from_nibbles(digits, false),
            4 => Self::from_nibbles(digits, true),
            6 => Self::from_pairs(digits, false),
            8 => Self::from_pairs(digits, true),
            _ => None,
        }
    }

    fn from_nibbles(digits: &[u8], has_alpha: bool) -> Option<Self> {
        Some(Self {
            red: expand_nibble(*digits.first()?)?,
            green: expand_nibble(*digits.get(1)?)?,
            blue: expand_nibble(*digits.get(2)?)?,
            alpha: if has_alpha {
                expand_nibble(*digits.get(3)?)?
            } else {
                OPAQUE
            },
        })
    }

    fn from_pairs(digits: &[u8], has_alpha: bool) -> Option<Self> {
        Some(Self {
            red: byte(digits, 0)?,
            green: byte(digits, 2)?,
            blue: byte(digits, 4)?,
            alpha: if has_alpha { byte(digits, 6)? } else { OPAQUE },
        })
    }

    pub(super) fn to_style(self, theme_background: Color) -> PluginStyle {
        let (red, green, blue) = self.blend(theme_background);
        let background = Color::Rgb {
            r: red,
            g: green,
            b: blue,
        };
        PluginStyle {
            foreground: contrast_foreground(red, green, blue),
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

fn expand_nibble(digit: u8) -> Option<u8> {
    hex_value(digit).map(|value| value * NIBBLE_SCALE)
}

fn byte(digits: &[u8], index: usize) -> Option<u8> {
    let high = hex_value(*digits.get(index)?)?;
    let low = hex_value(*digits.get(index.saturating_add(1))?)?;
    Some(high * HEX_RADIX + low)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
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

fn contrast_foreground(red: u8, green: u8, blue: u8) -> Color {
    let luminance = relative_luminance(red, green, blue);
    let black = (luminance + CONTRAST_OFFSET) / CONTRAST_OFFSET;
    let white = (1.0 + CONTRAST_OFFSET) / (luminance + CONTRAST_OFFSET);
    if black >= white {
        Color::Rgb {
            r: BLACK,
            g: BLACK,
            b: BLACK,
        }
    } else {
        Color::Rgb {
            r: WHITE,
            g: WHITE,
            b: WHITE,
        }
    }
}

fn relative_luminance(red: u8, green: u8, blue: u8) -> f64 {
    LUMINANCE_RED * linearize(red)
        + LUMINANCE_GREEN * linearize(green)
        + LUMINANCE_BLUE * linearize(blue)
}

fn linearize(channel: u8) -> f64 {
    let value = f64::from(channel) / f64::from(OPAQUE);
    if value <= GAMMA_THRESHOLD {
        value / GAMMA_DIVISOR
    } else {
        ((value + GAMMA_OFFSET) / GAMMA_SCALE).powf(GAMMA_EXPONENT)
    }
}
