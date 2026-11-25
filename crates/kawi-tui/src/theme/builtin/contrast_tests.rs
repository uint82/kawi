use super::test_support::contrast_ratio;
use super::*;

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
        (
            theme.bracket_match.foreground,
            theme.bracket_match.background,
        ),
    ] {
        assert!(contrast_ratio(foreground, background) >= 4.5);
    }
}

#[test]
fn bracket_match_is_subtle_and_readable_in_every_theme() {
    for name in Theme::names() {
        let theme = Theme::named(name).expect("theme");
        assert!(
            contrast_ratio(
                theme.bracket_match.foreground,
                theme.bracket_match.background
            ) >= 4.5,
            "{name} bracket match text is below 4.5:1 contrast"
        );
        let tint = contrast_ratio(theme.bracket_match.background, theme.base.background);
        assert!(
            (1.15..=2.0).contains(&tint),
            "{name} bracket match tint is {tint:.2}:1"
        );
    }
}
