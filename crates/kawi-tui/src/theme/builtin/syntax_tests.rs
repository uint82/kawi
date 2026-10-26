use super::palette::rgb;
use super::*;

#[test]
fn every_theme_colors_every_group() {
    for name in Theme::names() {
        let theme = Theme::named(name).expect("theme");
        for name in [
            "comment",
            "keyword",
            "string",
            "number",
            "constant",
            "function",
            "type",
            "variable",
            "property",
            "operator",
            "punctuation",
            "attribute",
            "tag",
            "invalid",
        ] {
            let color = theme
                .syntax
                .named(name)
                .and_then(|style| style.foreground)
                .expect("syntax color");
            assert_ne!(color, theme.base.background);
        }
    }
}

#[test]
fn default_base_scopes_use_kawi_attributes() {
    let theme = Theme::default();
    assert!(theme.syntax.named("comment").unwrap().italic);
    assert!(theme.syntax.named("string").unwrap().italic);
    assert!(theme.syntax.named("function").unwrap().bold);
    assert!(!theme.syntax.named("variable").unwrap().bold);
    assert!(!theme.syntax.named("punctuation").unwrap().italic);
}

#[test]
fn default_subcaptures_add_polish() {
    let theme = Theme::default();
    assert!(theme.syntax.named("comment.documentation").unwrap().italic);
    assert!(theme.syntax.named("function.macro").unwrap().italic);
    assert!(theme.syntax.named("keyword.return").unwrap().bold);
}

#[test]
fn default_syntax_uses_kawi_palette() {
    let theme = Theme::default();
    let expected = [
        ("comment", (114, 133, 130)),
        ("keyword", (255, 126, 135)),
        ("string", (168, 217, 138)),
        ("number", (201, 167, 255)),
        ("constant", (201, 167, 255)),
        ("function", (110, 208, 240)),
        ("type", (242, 194, 123)),
        ("variable", (231, 226, 214)),
        ("property", (240, 166, 184)),
        ("operator", (140, 158, 155)),
        ("punctuation", (140, 158, 155)),
        ("attribute", (111, 224, 200)),
        ("tag", (255, 126, 135)),
        ("invalid", (255, 107, 107)),
        ("variable.parameter", (240, 166, 184)),
        ("variable.member", (240, 166, 184)),
        ("function.method", (110, 208, 240)),
        ("function.macro", (111, 224, 200)),
        ("constant.builtin", (255, 184, 107)),
        ("type.builtin", (242, 194, 123)),
        ("keyword.return", (255, 126, 135)),
        ("keyword.import", (201, 167, 255)),
        ("tag.attribute", (240, 166, 184)),
    ];
    for (name, (r, g, b)) in expected {
        assert_eq!(
            theme.syntax.named(name).and_then(|style| style.foreground),
            Some(rgb(r, g, b)),
            "{name}"
        );
    }
}

#[test]
fn every_default_theme_key_is_configured() {
    let document: toml::Value = toml::from_str(DEFAULT_THEME).expect("embedded theme");
    let syntax = document
        .get("syntax")
        .and_then(toml::Value::as_table)
        .expect("syntax table");
    for key in syntax.keys() {
        assert!(
            kawi_syntax::registry().index_of(key).is_some(),
            "{key} is missing from the highlight registry"
        );
    }
}

#[test]
fn builtin_floats_use_the_editor_background() {
    for name in Theme::names() {
        let theme = Theme::named(name).expect("builtin theme");
        assert_eq!(theme.float_background, theme.base.background);
        assert_eq!(theme.code_background, theme.base.background);
    }
}
