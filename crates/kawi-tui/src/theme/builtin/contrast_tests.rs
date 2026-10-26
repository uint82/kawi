use crossterm::style::Color;

use super::*;

fn luminance(color: Color) -> f64 {
    let (r, g, b) = match color {
        Color::Rgb { r, g, b } => (r, g, b),
        _ => return 0.0,
    };
    let channel = |value: u8| {
        let value = f64::from(value) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}

fn contrast_ratio(foreground: Color, background: Color) -> f64 {
    let foreground = luminance(foreground);
    let background = luminance(background);
    let (lighter, darker) = if foreground > background {
        (foreground, background)
    } else {
        (background, foreground)
    };
    (lighter + 0.05) / (darker + 0.05)
}

#[test]
fn default_syntax_meets_body_contrast() {
    let theme = Theme::default();
    let document: toml::Value = toml::from_str(DEFAULT_THEME).expect("embedded theme");
    let syntax = document
        .get("syntax")
        .and_then(toml::Value::as_table)
        .expect("syntax table");
    for key in syntax.keys() {
        let Some(foreground) = theme.syntax.named(key).and_then(|style| style.foreground) else {
            continue;
        };
        assert!(
            contrast_ratio(foreground, theme.base.background) >= 4.5,
            "{key} is below 4.5:1 contrast"
        );
    }
}

#[test]
fn default_ui_meets_contrast() {
    let theme = Theme::default();
    for (foreground, background) in [
        (theme.base.foreground, theme.selection.background),
        (theme.base.foreground, theme.status.background),
        (theme.search_match.foreground, theme.search_match.background),
        (
            theme.search_current.foreground,
            theme.search_current.background,
        ),
        (theme.line_number, theme.base.background),
    ] {
        assert!(contrast_ratio(foreground, background) >= 4.5);
    }
}
