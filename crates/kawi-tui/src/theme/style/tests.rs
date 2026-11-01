use super::*;

fn style(foreground: Color) -> SyntaxStyle {
    SyntaxStyle::with_foreground(foreground)
}

fn registry_style(name: &str) -> SyntaxHighlight {
    registry().index_of(name).expect("registered capture")
}

#[test]
fn resolves_the_longest_prefix() {
    let mut map = BTreeMap::new();
    map.insert("keyword".to_string(), style(Color::Red));
    map.insert("keyword.return".to_string(), style(Color::Blue));
    assert_eq!(
        resolve_style(&map, "keyword.return").map(|style| style.foreground),
        Some(Some(Color::Blue))
    );
    assert_eq!(
        resolve_style(&map, "keyword.return.extra").map(|style| style.foreground),
        Some(Some(Color::Blue))
    );
}

#[test]
fn falls_back_to_a_parent_scope() {
    let mut map = BTreeMap::new();
    map.insert("keyword".to_string(), style(Color::Red));
    assert_eq!(
        resolve_style(&map, "keyword.return").map(|style| style.foreground),
        Some(Some(Color::Red))
    );
}

#[test]
fn carries_style_attributes() {
    let palette = parse_palette(BTreeMap::from([(
        "muted".to_string(),
        "#5c6370".to_string(),
    )]))
    .expect("palette");
    let styles = parse_styles(
        BTreeMap::from([
            (
                "comment".to_string(),
                StyleValue::Table(StyleTable {
                    fg: Some("muted".to_string()),
                    bg: None,
                    bold: false,
                    italic: true,
                    underline: true,
                }),
            ),
            ("keyword".to_string(), StyleValue::Color("red".to_string())),
            ("string".to_string(), StyleValue::Color("green".to_string())),
            ("number".to_string(), StyleValue::Color("blue".to_string())),
            (
                "constant".to_string(),
                StyleValue::Color("blue".to_string()),
            ),
            (
                "function".to_string(),
                StyleValue::Color("cyan".to_string()),
            ),
            ("type".to_string(), StyleValue::Color("yellow".to_string())),
            (
                "variable".to_string(),
                StyleValue::Color("white".to_string()),
            ),
            (
                "property".to_string(),
                StyleValue::Color("cyan".to_string()),
            ),
            (
                "operator".to_string(),
                StyleValue::Color("cyan".to_string()),
            ),
            (
                "punctuation".to_string(),
                StyleValue::Color("white".to_string()),
            ),
            (
                "attribute".to_string(),
                StyleValue::Color("yellow".to_string()),
            ),
            ("tag".to_string(), StyleValue::Color("red".to_string())),
            ("invalid".to_string(), StyleValue::Color("red".to_string())),
        ]),
        palette,
    )
    .expect("styles");
    let comment = styles.style(registry_style("comment")).expect("comment");
    assert_eq!(
        comment.foreground,
        Some(Color::Rgb {
            r: 92,
            g: 99,
            b: 112
        })
    );
    assert!(comment.italic);
    assert!(comment.underline);
    assert!(!comment.bold);
}
