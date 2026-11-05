use crossterm::style::Color;

const MAIN_TRC: f64 = 2.4;
const RED_COEFFICIENT: f64 = 0.212_672_9;
const GREEN_COEFFICIENT: f64 = 0.715_152_2;
const BLUE_COEFFICIENT: f64 = 0.072_175_0;
const NORMAL_BACKGROUND_EXPONENT: f64 = 0.56;
const NORMAL_TEXT_EXPONENT: f64 = 0.57;
const REVERSE_BACKGROUND_EXPONENT: f64 = 0.65;
const REVERSE_TEXT_EXPONENT: f64 = 0.62;
const BLACK_THRESHOLD: f64 = 0.022;
const BLACK_CLAMP_EXPONENT: f64 = 1.414;
const SCALE: f64 = 1.14;
const LOW_CLIP: f64 = 0.1;
const DELTA_Y_MIN: f64 = 0.0005;
const OFFSET: f64 = 0.027;
const MAX_PERCENT: f64 = 100.0;
const MAX_CHANNEL: f64 = 255.0;
const BLACK_CHANNEL: u8 = 0;
const WHITE_CHANNEL: u8 = 255;
const BLACK: [u8; 3] = [BLACK_CHANNEL; 3];
const WHITE: [u8; 3] = [WHITE_CHANNEL; 3];

pub(super) fn foreground(red: u8, green: u8, blue: u8) -> Color {
    let background = [red, green, blue];
    if prefers_black(background) {
        rgb(BLACK)
    } else {
        rgb(WHITE)
    }
}

fn prefers_black(background: [u8; 3]) -> bool {
    let black = lightness_contrast(BLACK, background).abs();
    let white = lightness_contrast(WHITE, background).abs();
    black >= white
}

fn lightness_contrast(text: [u8; 3], background: [u8; 3]) -> f64 {
    let text_luminance = clamp_luminance(screen_luminance(text));
    let background_luminance = clamp_luminance(screen_luminance(background));
    if (background_luminance - text_luminance).abs() < DELTA_Y_MIN {
        return 0.0;
    }
    let contrast = if background_luminance > text_luminance {
        polarity_contrast(
            background_luminance,
            text_luminance,
            NORMAL_BACKGROUND_EXPONENT,
            NORMAL_TEXT_EXPONENT,
        )
    } else {
        polarity_contrast(
            background_luminance,
            text_luminance,
            REVERSE_BACKGROUND_EXPONENT,
            REVERSE_TEXT_EXPONENT,
        )
    };
    scale_contrast(contrast)
}

fn polarity_contrast(
    background: f64,
    text: f64,
    background_exponent: f64,
    text_exponent: f64,
) -> f64 {
    (background.powf(background_exponent) - text.powf(text_exponent)) * SCALE
}

fn scale_contrast(contrast: f64) -> f64 {
    if contrast.abs() < LOW_CLIP {
        0.0
    } else if contrast > 0.0 {
        (contrast - OFFSET) * MAX_PERCENT
    } else {
        (contrast + OFFSET) * MAX_PERCENT
    }
}

fn screen_luminance([red, green, blue]: [u8; 3]) -> f64 {
    RED_COEFFICIENT * linear_channel(red)
        + GREEN_COEFFICIENT * linear_channel(green)
        + BLUE_COEFFICIENT * linear_channel(blue)
}

fn linear_channel(channel: u8) -> f64 {
    (f64::from(channel) / MAX_CHANNEL).powf(MAIN_TRC)
}

fn clamp_luminance(luminance: f64) -> f64 {
    if luminance < BLACK_THRESHOLD {
        luminance + (BLACK_THRESHOLD - luminance).powf(BLACK_CLAMP_EXPONENT)
    } else {
        luminance
    }
}

fn rgb([red, green, blue]: [u8; 3]) -> Color {
    Color::Rgb {
        r: red,
        g: green,
        b: blue,
    }
}

#[cfg(test)]
mod tests {
    use super::{foreground, lightness_contrast};
    use crossterm::style::Color;

    const BLACK: Color = Color::Rgb { r: 0, g: 0, b: 0 };
    const WHITE: Color = Color::Rgb {
        r: 255,
        g: 255,
        b: 255,
    };

    fn assert_close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 0.01, "{actual} != {expected}");
    }

    #[test]
    fn uses_white_on_red_backgrounds() {
        assert_eq!(foreground(255, 0, 0), WHITE);
        assert_eq!(foreground(251, 73, 52), WHITE);
    }

    #[test]
    fn uses_black_on_bright_green_and_yellow() {
        assert_eq!(foreground(0, 255, 0), BLACK);
        assert_eq!(foreground(255, 255, 0), BLACK);
        assert_eq!(foreground(255, 165, 0), BLACK);
    }

    #[test]
    fn uses_white_on_dark_and_blue_backgrounds() {
        assert_eq!(foreground(0, 0, 0), WHITE);
        assert_eq!(foreground(0, 0, 255), WHITE);
        assert_eq!(foreground(128, 0, 128), WHITE);
    }

    #[test]
    fn matches_apca_reference_values() {
        assert_close(
            lightness_contrast([174, 25, 50], [224, 223, 222]),
            63.842_885_820_069_01,
        );
        assert_close(
            lightness_contrast([209, 111, 21], [224, 223, 222]),
            43.342_135_468_280_915,
        );
        assert_close(
            lightness_contrast([85, 139, 207], [224, 223, 222]),
            43.783_448_119_859_97,
        );
    }

    #[test]
    fn keeps_reverse_polarity_signed() {
        assert!(lightness_contrast([255; 3], [0; 3]) < 0.0);
    }
}
