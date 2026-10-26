use crossterm::style::Color;

use super::*;

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
fn defaults_use_kawi_base() {
    let theme = Theme::default();
    assert_eq!(theme.name, "default");
    assert_eq!(
        theme.base.background,
        Color::Rgb {
            r: 16,
            g: 23,
            b: 26
        }
    );
    assert_eq!(
        theme.base.foreground,
        Color::Rgb {
            r: 231,
            g: 226,
            b: 214
        }
    );
}

#[test]
fn defaults_use_kawi_search_palette() {
    let theme = Theme::default();
    assert_eq!(
        theme.search_match.foreground,
        Color::Rgb {
            r: 16,
            g: 23,
            b: 26
        }
    );
    assert_eq!(
        theme.search_match.background,
        Color::Rgb {
            r: 242,
            g: 194,
            b: 123
        }
    );
    assert_eq!(
        theme.search_current.foreground,
        Color::Rgb {
            r: 16,
            g: 23,
            b: 26
        }
    );
    assert_eq!(
        theme.search_current.background,
        Color::Rgb {
            r: 255,
            g: 184,
            b: 107
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
fn default_selection_uses_kawi_surface() {
    let theme = Theme::default();
    assert_eq!(
        theme.selection.foreground,
        Color::Rgb {
            r: 231,
            g: 226,
            b: 214
        }
    );
    assert_eq!(
        theme.selection.background,
        Color::Rgb {
            r: 44,
            g: 62,
            b: 67
        }
    );
}

#[test]
fn default_status_uses_kawi_surface() {
    let theme = Theme::default();
    assert_eq!(
        theme.status.foreground,
        Color::Rgb {
            r: 231,
            g: 226,
            b: 214
        }
    );
    assert_eq!(
        theme.status.background,
        Color::Rgb {
            r: 35,
            g: 49,
            b: 54
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
fn default_differs_from_gruvbox_dark() {
    let default = Theme::default();
    let gruvbox = Theme::named("gruvbox-dark").expect("theme");
    assert_ne!(default.base.background, gruvbox.base.background);
    assert_ne!(
        default
            .syntax
            .named("function")
            .and_then(|style| style.foreground),
        gruvbox
            .syntax
            .named("function")
            .and_then(|style| style.foreground)
    );
}

#[test]
fn cobalt_uses_a_deep_blue_palette() {
    let theme = Theme::named("cobalt").expect("theme");
    assert_eq!(
        theme.base.background,
        Color::Rgb {
            r: 10,
            g: 27,
            b: 46
        }
    );
    assert_eq!(
        theme
            .syntax
            .named("keyword")
            .and_then(|style| style.foreground),
        Some(Color::Rgb {
            r: 245,
            g: 215,
            b: 110
        })
    );
    assert_eq!(
        theme
            .syntax
            .named("function")
            .and_then(|style| style.foreground),
        Some(Color::Rgb {
            r: 102,
            g: 217,
            b: 239
        })
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
