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
