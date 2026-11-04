use super::*;

#[test]
fn key_color_uses_the_theme_keyword() {
    let theme = Theme::default();
    let expected = theme
        .syntax
        .named("keyword")
        .and_then(|style| style.foreground)
        .expect("keyword color");
    assert_eq!(key_color(&theme), expected);
    assert_ne!(key_color(&theme), theme.base.foreground);
}
