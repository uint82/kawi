use super::support::{highlight, name_at, name_of};
use super::*;

#[test]
fn highlights_css_selectors_and_combinators() {
    let text = "article.card > a[data-kind^=\"hero\"]:hover::before,\n#hero ~ * + &,\ninput[data-role=search] {\n  color: red;\n}\n";
    let spans = highlight(LanguageId::Css, text, 0, 5);
    assert_eq!(name_of(&spans, text, 0, "article"), Some("tag"));
    assert_eq!(name_of(&spans, text, 0, "card"), Some("type"));
    assert_eq!(name_of(&spans, text, 0, "data-kind"), Some("tag.attribute"));
    assert_eq!(name_of(&spans, text, 0, "^="), Some("operator"));
    assert_eq!(name_of(&spans, text, 0, "\"hero\""), Some("string"));
    assert_eq!(name_of(&spans, text, 0, "hover"), Some("attribute"));
    assert_eq!(name_of(&spans, text, 0, "before"), Some("attribute"));
    assert_eq!(name_of(&spans, text, 0, ">"), Some("operator"));
    assert_eq!(name_of(&spans, text, 1, "hero"), Some("constant"));
    assert_eq!(name_of(&spans, text, 1, "~"), Some("operator"));
    assert_eq!(name_of(&spans, text, 1, "+"), Some("operator"));
    assert_eq!(name_of(&spans, text, 1, "&"), Some("character.special"));
    assert_eq!(name_of(&spans, text, 1, "*"), Some("character.special"));
    assert_eq!(name_of(&spans, text, 2, "input"), Some("tag"));
    assert_eq!(name_of(&spans, text, 2, "data-role"), Some("tag.attribute"));
    assert_eq!(name_of(&spans, text, 2, "="), Some("operator"));
    assert_eq!(name_of(&spans, text, 2, "search"), Some("string"));
    assert_eq!(name_of(&spans, text, 3, "color"), Some("property"));
}

#[test]
fn highlights_css_custom_properties_and_values() {
    let text = ":root {\n  --accent: #ff6600;\n  --gap: 1.5rem;\n  color: var(--accent, red);\n  margin: calc(100% - 2rem) !important;\n}\n";
    let spans = highlight(LanguageId::Css, text, 0, 6);
    assert_eq!(name_of(&spans, text, 1, "accent"), Some("variable"));
    assert_eq!(name_of(&spans, text, 1, "ff6600"), Some("string"));
    assert_eq!(name_of(&spans, text, 2, "gap"), Some("variable"));
    assert_eq!(name_of(&spans, text, 2, "1.5"), Some("number.float"));
    assert_eq!(name_of(&spans, text, 2, "rem"), Some("string"));
    assert_eq!(name_of(&spans, text, 3, "color"), Some("property"));
    assert_eq!(name_of(&spans, text, 3, "var"), Some("function"));
    assert_eq!(name_of(&spans, text, 3, "accent"), Some("variable"));
    assert_eq!(name_of(&spans, text, 4, "margin"), Some("property"));
    assert_eq!(name_of(&spans, text, 4, "calc"), Some("function"));
    assert_eq!(name_of(&spans, text, 4, "100"), Some("number"));
    assert_eq!(name_of(&spans, text, 4, "%"), Some("string"));
    assert_eq!(name_of(&spans, text, 4, "-"), Some("operator"));
    assert_eq!(name_of(&spans, text, 4, "2"), Some("number"));
    assert_eq!(name_of(&spans, text, 4, "rem"), Some("string"));
    assert_eq!(
        name_of(&spans, text, 4, "important"),
        Some("keyword.modifier")
    );
}

#[test]
fn highlights_css_at_rules_media_and_keyframes() {
    let text = "@charset \"utf-8\";\n@import url(\"theme.css\") screen and (min-width: 600px);\n@namespace svg url(\"http://www.w3.org/2000/svg\");\n@layer components;\n@media not all and (min-width: 40rem) {\n  .box { color: red; }\n}\n@supports (display: grid) and (gap: 1rem) {\n  .grid { gap: 1rem; }\n}\n@keyframes fade {\n  from { opacity: 0; }\n  to { opacity: 1; }\n}\n@media only screen and (color) {\n  .box { color: red; }\n}\n@media (hover) or (pointer: fine) {\n  .box { color: red; }\n}\n";
    let spans = highlight(LanguageId::Css, text, 0, 20);
    assert_eq!(
        name_of(&spans, text, 0, "@charset"),
        Some("keyword.directive")
    );
    assert_eq!(name_of(&spans, text, 0, "utf-8"), Some("string"));
    assert_eq!(name_of(&spans, text, 1, "@import"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 1, "url"), Some("function"));
    assert_eq!(name_of(&spans, text, 1, "theme.css"), Some("string"));
    assert_eq!(name_of(&spans, text, 1, "and"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 1, "min-width"), Some("property"));
    assert_eq!(name_of(&spans, text, 1, "600"), Some("number"));
    assert_eq!(name_of(&spans, text, 1, "px"), Some("string"));
    assert_eq!(
        name_of(&spans, text, 2, "@namespace"),
        Some("keyword.directive")
    );
    assert_eq!(name_of(&spans, text, 2, "svg"), Some("module"));
    assert_eq!(name_of(&spans, text, 2, "url"), Some("function"));
    assert_eq!(
        name_of(&spans, text, 3, "@layer"),
        Some("keyword.directive")
    );
    assert_eq!(
        name_of(&spans, text, 4, "@media"),
        Some("keyword.directive")
    );
    assert_eq!(name_of(&spans, text, 4, "not"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 4, "and"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 4, "min-width"), Some("property"));
    assert_eq!(name_of(&spans, text, 4, "40"), Some("number"));
    assert_eq!(name_of(&spans, text, 4, "rem"), Some("string"));
    assert_eq!(
        name_of(&spans, text, 7, "@supports"),
        Some("keyword.directive")
    );
    assert_eq!(name_of(&spans, text, 7, "display"), Some("property"));
    assert_eq!(name_of(&spans, text, 7, "and"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 7, "gap"), Some("property"));
    assert_eq!(name_of(&spans, text, 7, "1"), Some("number"));
    assert_eq!(name_of(&spans, text, 7, "rem"), Some("string"));
    assert_eq!(
        name_of(&spans, text, 10, "@keyframes"),
        Some("keyword.directive")
    );
    assert_eq!(name_of(&spans, text, 10, "fade"), Some("variable"));
    assert_eq!(name_of(&spans, text, 11, "from"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 11, "opacity"), Some("property"));
    assert_eq!(name_of(&spans, text, 11, "0"), Some("number"));
    assert_eq!(name_of(&spans, text, 12, "to"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 12, "1"), Some("number"));
    assert_eq!(name_of(&spans, text, 14, "only"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 14, "and"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 17, "or"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 17, "pointer"), Some("property"));
}

#[test]
fn highlights_css_functions_literals_and_escapes() {
    let text = "/* theme */\na {\n  width: calc(100% - 2rem);\n  background: rgba(0, 0, 0, 0.5) url(\"img.png\") #fff;\n  height: calc(2 * 3);\n  content: \"a\\a b\";\n}\n";
    let spans = highlight(LanguageId::Css, text, 0, 7);
    assert_eq!(name_of(&spans, text, 0, "theme"), Some("comment"));
    assert_eq!(name_of(&spans, text, 1, "a"), Some("tag"));
    assert_eq!(name_of(&spans, text, 2, "width"), Some("property"));
    assert_eq!(name_of(&spans, text, 2, "calc"), Some("function"));
    assert_eq!(name_of(&spans, text, 2, "100"), Some("number"));
    assert_eq!(name_of(&spans, text, 2, "%"), Some("string"));
    assert_eq!(name_of(&spans, text, 2, "-"), Some("operator"));
    assert_eq!(name_of(&spans, text, 2, "2"), Some("number"));
    assert_eq!(name_of(&spans, text, 2, "rem"), Some("string"));
    assert_eq!(name_of(&spans, text, 3, "background"), Some("property"));
    assert_eq!(name_of(&spans, text, 3, "rgba"), Some("function"));
    assert_eq!(name_of(&spans, text, 3, "0"), Some("number"));
    assert_eq!(name_of(&spans, text, 3, "0.5"), Some("number.float"));
    assert_eq!(name_of(&spans, text, 3, "url"), Some("function"));
    assert_eq!(name_of(&spans, text, 3, "img.png"), Some("string"));
    assert_eq!(name_of(&spans, text, 3, "fff"), Some("string"));
    assert_eq!(name_of(&spans, text, 4, "height"), Some("property"));
    assert_eq!(name_of(&spans, text, 4, "calc"), Some("function"));
    assert_eq!(name_of(&spans, text, 4, "2"), Some("number"));
    assert_eq!(name_of(&spans, text, 4, "*"), Some("operator"));
    assert_eq!(name_of(&spans, text, 4, "3"), Some("number"));
    assert_eq!(name_of(&spans, text, 5, "content"), Some("property"));

    let escape = text.find("\\a").expect("escape");
    let prefix = &text[..escape];
    let row = prefix.matches('\n').count();
    let column = prefix.len() - prefix.rfind('\n').map_or(0, |index| index + 1);
    assert_eq!(
        name_at(&spans, row, column),
        Some("string.escape"),
        "escape sequence"
    );

    let brace = text
        .lines()
        .nth(1)
        .and_then(|line| line.find('{'))
        .expect("brace");
    assert_eq!(name_at(&spans, 1, brace), Some("punctuation.bracket"));
    let semicolon = text
        .lines()
        .nth(2)
        .and_then(|line| line.rfind(';'))
        .expect("semicolon");
    assert_eq!(name_at(&spans, 2, semicolon), Some("punctuation.delimiter"));
}

#[test]
fn highlights_css_modern_pseudo_selectors() {
    let text = ".card:not(.disabled):is(.active, #hero) > :nth-child(2n + 1) {\n  color: red;\n}\n";
    let spans = highlight(LanguageId::Css, text, 0, 3);
    assert_eq!(name_of(&spans, text, 0, "card"), Some("type"));
    assert_eq!(name_of(&spans, text, 0, "not"), Some("attribute"));
    assert_eq!(name_of(&spans, text, 0, "disabled"), Some("type"));
    let is = text
        .lines()
        .next()
        .and_then(|line| line.find(":is"))
        .expect("is")
        + 1;
    assert_eq!(name_at(&spans, 0, is), Some("attribute"));
    assert_eq!(name_of(&spans, text, 0, "active"), Some("type"));
    assert_eq!(name_of(&spans, text, 0, "hero"), Some("constant"));
    assert_eq!(name_of(&spans, text, 0, "nth-child"), Some("attribute"));
    assert_eq!(name_of(&spans, text, 0, ">"), Some("operator"));
    assert_eq!(name_of(&spans, text, 1, "color"), Some("property"));
}
