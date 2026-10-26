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
fn registry_resolves_all_names() {
    for name in Theme::names() {
        assert_eq!(
            Theme::named(name).map(|theme| theme.name),
            Some(name.to_string())
        );
    }
    assert!(Theme::named("nope").is_none());
}

#[test]
fn defaults_follow_neovim_base() {
    let theme = Theme::default();
    assert_eq!(theme.name, "default");
    assert_eq!(
        theme.base.background,
        Color::Rgb {
            r: 40,
            g: 40,
            b: 40
        }
    );
    assert_eq!(
        theme.base.foreground,
        Color::Rgb {
            r: 235,
            g: 219,
            b: 178
        }
    );
}

#[test]
fn defaults_follow_neovim_palette() {
    let theme = Theme::default();
    assert_eq!(
        theme.search_match.foreground,
        Color::Rgb {
            r: 40,
            g: 40,
            b: 40
        }
    );
    assert_eq!(
        theme.search_match.background,
        Color::Rgb {
            r: 250,
            g: 189,
            b: 47
        }
    );
    assert_eq!(
        theme.search_current.foreground,
        Color::Rgb {
            r: 40,
            g: 40,
            b: 40
        }
    );
    assert_eq!(
        theme.search_current.background,
        Color::Rgb {
            r: 254,
            g: 128,
            b: 25
        }
    );
}

#[test]
fn current_match_differs_from_background() {
    let theme = Theme::default();
    assert_ne!(
        theme.search_match.background,
        theme.search_current.background
    );
}

#[test]
fn substitute_matches_search_by_default() {
    for name in Theme::names() {
        let theme = Theme::named(name).expect("theme");
        assert_eq!(theme.substitute.foreground, theme.search_match.foreground);
        assert_eq!(theme.substitute.background, theme.search_match.background);
    }
}

#[test]
fn default_selection_uses_gruvbox_surfaces() {
    let theme = Theme::default();
    assert_eq!(
        theme.selection.foreground,
        Color::Rgb {
            r: 235,
            g: 219,
            b: 178
        }
    );
    assert_eq!(
        theme.selection.background,
        Color::Rgb {
            r: 102,
            g: 92,
            b: 84
        }
    );
}

#[test]
fn default_status_uses_gruvbox_surface() {
    let theme = Theme::default();
    assert_eq!(
        theme.status.foreground,
        Color::Rgb {
            r: 235,
            g: 219,
            b: 178
        }
    );
    assert_eq!(
        theme.status.background,
        Color::Rgb {
            r: 80,
            g: 73,
            b: 69
        }
    );
}

#[test]
fn gruvbox_dark_selection_and_status() {
    let theme = Theme::named("gruvbox-dark").expect("theme");
    assert_eq!(
        theme.selection.background,
        Color::Rgb {
            r: 102,
            g: 92,
            b: 84
        }
    );
    assert_eq!(
        theme.status.background,
        Color::Rgb {
            r: 80,
            g: 73,
            b: 69
        }
    );
}

#[test]
fn gruvbox_dark_matches_user_neovim() {
    let theme = Theme::named("gruvbox-dark").expect("theme");
    assert_eq!(
        theme.base.background,
        Color::Rgb {
            r: 40,
            g: 40,
            b: 40
        }
    );
    assert_eq!(
        theme.base.foreground,
        Color::Rgb {
            r: 235,
            g: 219,
            b: 178
        }
    );
    assert_eq!(
        theme.search_match.background,
        Color::Rgb {
            r: 250,
            g: 189,
            b: 47
        }
    );
    assert_eq!(
        theme.search_current.background,
        Color::Rgb {
            r: 254,
            g: 128,
            b: 25
        }
    );
}

#[test]
fn one_dark_preserves_the_previous_default() {
    let theme = Theme::named("one-dark").expect("theme");
    assert_eq!(
        theme.base.background,
        Color::Rgb {
            r: 20,
            g: 22,
            b: 27
        }
    );
    assert_eq!(
        theme
            .syntax
            .named("keyword")
            .and_then(|style| style.foreground),
        Some(Color::Rgb {
            r: 198,
            g: 120,
            b: 221
        })
    );
}

#[test]
fn default_syntax_meets_minimum_ui_contrast() {
    let theme = Theme::default();
    let document: toml::Value = toml::from_str(GRUVBOX_NIGHT).expect("embedded theme");
    let syntax = document
        .get("syntax")
        .and_then(toml::Value::as_table)
        .expect("syntax table");
    for key in syntax.keys() {
        let Some(foreground) = theme.syntax.named(key).and_then(|style| style.foreground) else {
            continue;
        };
        assert!(
            contrast_ratio(foreground, theme.base.background) >= 3.0,
            "{key} is below 3:1 contrast"
        );
    }
}

#[test]
fn gruvbox_light_uses_dark_accents() {
    let theme = Theme::named("gruvbox-light").expect("theme");
    assert_eq!(
        theme.base.background,
        Color::Rgb {
            r: 251,
            g: 241,
            b: 199
        }
    );
    assert_eq!(
        theme.search_match.background,
        Color::Rgb {
            r: 181,
            g: 118,
            b: 20
        }
    );
    assert_eq!(
        theme.search_current.background,
        Color::Rgb {
            r: 175,
            g: 58,
            b: 3
        }
    );
}
