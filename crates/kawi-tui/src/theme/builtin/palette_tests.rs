use super::test_support::rgb;
use super::*;

#[test]
fn gruvbox_dark_selection_and_status() {
    let theme = Theme::named("gruvbox-dark").expect("theme");
    assert_eq!(theme.selection.background, rgb(102, 92, 84));
    assert_eq!(theme.status.background, rgb(80, 73, 69));
}

#[test]
fn gruvbox_dark_matches_user_neovim() {
    let theme = Theme::named("gruvbox-dark").expect("theme");
    assert_eq!(theme.base.background, rgb(40, 40, 40));
    assert_eq!(theme.base.foreground, rgb(235, 219, 178));
    assert_eq!(theme.search_match.background, rgb(250, 189, 47));
    assert_eq!(theme.search_current.background, rgb(254, 128, 25));
}

#[test]
fn gruvbox_special_key_matches_user_neovim() {
    let dark = Theme::named("gruvbox-dark").expect("theme");
    assert_eq!(dark.special_key, rgb(168, 153, 132));
    let light = Theme::named("gruvbox-light").expect("theme");
    assert_eq!(light.special_key, rgb(124, 111, 100));
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
    assert_eq!(theme.base.background, rgb(10, 27, 46));
    assert_eq!(
        theme
            .syntax
            .named("keyword")
            .and_then(|style| style.foreground),
        Some(rgb(245, 215, 110))
    );
    assert_eq!(
        theme
            .syntax
            .named("function")
            .and_then(|style| style.foreground),
        Some(rgb(102, 217, 239))
    );
}

#[test]
fn classic_uses_the_dos_vga_palette() {
    let theme = Theme::named("classic").expect("theme");
    assert_eq!(theme.base.background, rgb(0, 0, 170));
    assert_eq!(theme.base.foreground, rgb(170, 170, 170));
    assert_eq!(theme.selection.background, rgb(0, 0, 85));
    assert_eq!(theme.status.background, rgb(170, 170, 170));
    assert_eq!(theme.bracket_match.foreground, rgb(255, 255, 85));
    assert_eq!(theme.bracket_match.background, rgb(0, 0, 255));
    for (name, expected) in [
        ("keyword", (255, 255, 255)),
        ("string", (255, 255, 85)),
        ("function", (85, 255, 255)),
        ("comment", (85, 170, 170)),
    ] {
        let foreground = theme
            .syntax
            .named(name)
            .and_then(|style| style.foreground)
            .expect("syntax color");
        assert_eq!(
            foreground,
            rgb(expected.0, expected.1, expected.2),
            "{name}"
        );
    }
}

#[test]
fn borland_uses_the_turbo_pascal_palette() {
    let theme = Theme::named("borland").expect("theme");
    assert_eq!(theme.base.background, rgb(0, 0, 168));
    assert_eq!(theme.base.foreground, rgb(255, 255, 87));
    assert_eq!(theme.selection.foreground, rgb(0, 0, 168));
    assert_eq!(theme.selection.background, rgb(168, 168, 168));
    assert_eq!(theme.status.foreground, rgb(0, 0, 0));
    assert_eq!(theme.status.background, rgb(170, 170, 170));
    assert_eq!(theme.bracket_match.foreground, rgb(255, 255, 87));
    assert_eq!(theme.bracket_match.background, rgb(0, 84, 168));
    for (name, expected) in [
        ("keyword", (255, 255, 255)),
        ("string", (255, 255, 87)),
        ("function", (255, 255, 255)),
        ("comment", (168, 168, 168)),
        ("type", (87, 255, 255)),
        ("number", (87, 255, 255)),
        ("constant", (87, 255, 255)),
        ("variable", (255, 255, 87)),
        ("operator", (168, 168, 168)),
        ("attribute", (87, 255, 87)),
        ("tag", (87, 255, 255)),
    ] {
        let foreground = theme
            .syntax
            .named(name)
            .and_then(|style| style.foreground)
            .expect("syntax color");
        assert_eq!(
            foreground,
            rgb(expected.0, expected.1, expected.2),
            "{name}"
        );
    }
}

#[test]
fn borland_keeps_the_classic_turbo_identity() {
    let borland = Theme::named("borland").expect("theme");
    let classic = Theme::named("classic").expect("theme");
    assert_ne!(borland.base.foreground, classic.base.foreground);
    assert_ne!(
        borland
            .syntax
            .named("type")
            .and_then(|style| style.foreground),
        classic
            .syntax
            .named("type")
            .and_then(|style| style.foreground)
    );
}

#[test]
fn one_dark_preserves_the_previous_default() {
    let theme = Theme::named("one-dark").expect("theme");
    assert_eq!(theme.base.background, rgb(20, 22, 27));
    assert_eq!(
        theme
            .syntax
            .named("keyword")
            .and_then(|style| style.foreground),
        Some(rgb(198, 120, 221))
    );
}

#[test]
fn gruvbox_light_uses_dark_accents() {
    let theme = Theme::named("gruvbox-light").expect("theme");
    assert_eq!(theme.base.background, rgb(251, 241, 199));
    assert_eq!(theme.search_match.background, rgb(181, 118, 20));
    assert_eq!(theme.search_current.background, rgb(175, 58, 3));
}
