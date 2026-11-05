use super::super::color_preview::Rgba;

const MAX_CHANNEL: f32 = 255.0;
const PERCENT: f32 = 100.0;
const HUE_SECTORS: f32 = 6.0;
const SECTOR_DEGREES: f32 = 60.0;
const THIRD: f32 = 1.0 / 3.0;
const TWO_THIRDS: f32 = 2.0 / 3.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Hsla {
    pub(super) hue: f32,
    pub(super) saturation: f32,
    pub(super) lightness: f32,
    pub(super) alpha: f32,
}

impl Hsla {
    pub(super) fn from_rgba(color: Rgba) -> Self {
        let (red, green, blue) = channels(color);
        let maximum = red.max(green).max(blue);
        let minimum = red.min(green).min(blue);
        let lightness = (maximum + minimum) / 2.0;
        let delta = maximum - minimum;
        Self {
            hue: hue(red, green, blue, maximum, delta),
            saturation: saturation(maximum, minimum, lightness, delta) * PERCENT,
            lightness: lightness * PERCENT,
            alpha: alpha(color.alpha),
        }
    }

    pub(super) fn to_rgba(self) -> Rgba {
        let hue = self.hue.rem_euclid(360.0) / 360.0;
        let saturation = (self.saturation / PERCENT).clamp(0.0, 1.0);
        let lightness = (self.lightness / PERCENT).clamp(0.0, 1.0);
        let (red, green, blue) = if saturation == 0.0 {
            (lightness, lightness, lightness)
        } else {
            let upper = if lightness < 0.5 {
                lightness * (1.0 + saturation)
            } else {
                lightness + saturation - lightness * saturation
            };
            let lower = 2.0 * lightness - upper;
            (
                hue_channel(lower, upper, hue + THIRD),
                hue_channel(lower, upper, hue),
                hue_channel(lower, upper, hue - THIRD),
            )
        };
        Rgba::new(
            channel(red),
            channel(green),
            channel(blue),
            channel(self.alpha / PERCENT),
        )
    }
}

fn channels(color: Rgba) -> (f32, f32, f32) {
    (
        f32::from(color.red) / MAX_CHANNEL,
        f32::from(color.green) / MAX_CHANNEL,
        f32::from(color.blue) / MAX_CHANNEL,
    )
}

fn alpha(value: u8) -> f32 {
    f32::from(value) / MAX_CHANNEL * PERCENT
}

fn saturation(maximum: f32, minimum: f32, lightness: f32, delta: f32) -> f32 {
    if delta == 0.0 {
        0.0
    } else if lightness > 0.5 {
        delta / (2.0 - maximum - minimum)
    } else {
        delta / (maximum + minimum)
    }
}

fn hue(red: f32, green: f32, blue: f32, maximum: f32, delta: f32) -> f32 {
    if delta == 0.0 {
        return 0.0;
    }
    let sector = if maximum == red {
        ((green - blue) / delta + HUE_SECTORS) % HUE_SECTORS
    } else if maximum == green {
        (blue - red) / delta + 2.0
    } else {
        (red - green) / delta + 4.0
    };
    sector * SECTOR_DEGREES
}

fn hue_channel(lower: f32, upper: f32, hue: f32) -> f32 {
    let hue = hue.rem_euclid(1.0);
    if hue < HUE_SECTORS.recip() {
        lower + (upper - lower) * HUE_SECTORS * hue
    } else if hue < 0.5 {
        upper
    } else if hue < TWO_THIRDS {
        lower + (upper - lower) * (TWO_THIRDS - hue) * HUE_SECTORS
    } else {
        lower
    }
}

fn channel(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * MAX_CHANNEL).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_primary_colors() {
        let red = Hsla::from_rgba(Rgba::new(255, 0, 0, 255));
        assert_eq!(red.hue, 0.0);
        assert_eq!(red.saturation, 100.0);
        assert_eq!(red.lightness, 50.0);
        let green = Hsla::from_rgba(Rgba::new(0, 255, 0, 255));
        assert_eq!(green.hue, 120.0);
        let blue = Hsla::from_rgba(Rgba::new(0, 0, 255, 255));
        assert_eq!(blue.hue, 240.0);
    }

    #[test]
    fn converts_grayscale() {
        let white = Hsla::from_rgba(Rgba::new(255, 255, 255, 255));
        assert_eq!(white.saturation, 0.0);
        assert_eq!(white.lightness, 100.0);
        assert_eq!(white.to_rgba(), Rgba::new(255, 255, 255, 255));
        let black = Hsla::from_rgba(Rgba::new(0, 0, 0, 128));
        assert!((black.alpha - 50.19608).abs() < 0.001);
        assert_eq!(black.to_rgba(), Rgba::new(0, 0, 0, 128));
    }

    #[test]
    fn round_trips_channels() {
        for color in [
            Rgba::new(204, 34, 51, 255),
            Rgba::new(12, 200, 128, 64),
            Rgba::new(255, 128, 0, 0),
        ] {
            assert_eq!(Hsla::from_rgba(color).to_rgba(), color);
        }
    }

    #[test]
    fn wraps_hue_sectors() {
        let mut color = Hsla::from_rgba(Rgba::new(255, 0, 0, 255));
        color.hue = 360.0;
        assert_eq!(color.to_rgba(), Rgba::new(255, 0, 0, 255));
        color.hue = 480.0;
        assert_eq!(color.to_rgba(), Rgba::new(0, 255, 0, 255));
    }
}
