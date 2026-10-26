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
fn default_base_scopes_use_gruvbox_attributes() {
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
fn default_syntax_matches_neovim_gruvbox() {
    let theme = Theme::default();
    let expected = [
        ("comment", (146, 131, 116)),
        ("keyword", (251, 73, 52)),
        ("string", (184, 187, 38)),
        ("number", (211, 134, 155)),
        ("constant", (211, 134, 155)),
        ("function", (184, 187, 38)),
        ("type", (250, 189, 47)),
        ("variable", (235, 219, 178)),
        ("property", (131, 165, 152)),
        ("operator", (254, 128, 25)),
        ("punctuation", (254, 128, 25)),
        ("attribute", (142, 192, 124)),
        ("tag", (254, 128, 25)),
        ("invalid", (251, 73, 52)),
        ("variable.parameter", (131, 165, 152)),
        ("variable.member", (131, 165, 152)),
        ("function.method", (184, 187, 38)),
        ("function.macro", (142, 192, 124)),
        ("constant.builtin", (254, 128, 25)),
        ("type.builtin", (250, 189, 47)),
        ("keyword.return", (251, 73, 52)),
        ("keyword.import", (142, 192, 124)),
        ("tag.attribute", (131, 165, 152)),
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
    let document: toml::Value = toml::from_str(GRUVBOX_NIGHT).expect("embedded theme");
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
