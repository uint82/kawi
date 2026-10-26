use crossterm::style::Color;
use std::collections::BTreeMap;

use super::super::SyntaxStyle;

pub(super) fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb { r, g, b }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Palette {
    comment: Color,
    keyword: Color,
    string: Color,
    number: Color,
    constant: Color,
    function: Color,
    type_name: Color,
    variable: Color,
    property: Color,
    operator: Color,
    punctuation: Color,
    attribute: Color,
    tag: Color,
    invalid: Color,
}

impl Palette {
    pub(super) fn styles(self) -> BTreeMap<String, SyntaxStyle> {
        let mut styles = BTreeMap::new();
        add_base_styles(&mut styles, self);
        add_subcapture_styles(&mut styles, self);
        styles
    }
}

fn add_base_styles(styles: &mut BTreeMap<String, SyntaxStyle>, palette: Palette) {
    add_style(styles, "comment", palette.comment);
    add_style(styles, "keyword", palette.keyword);
    add_style(styles, "string", palette.string);
    add_style(styles, "escape", palette.string);
    add_style(styles, "character", palette.string);
    add_style(styles, "number", palette.number);
    add_style(styles, "constant", palette.constant);
    add_style(styles, "boolean", palette.constant);
    add_style(styles, "special", palette.constant);
    add_style(styles, "function", palette.function);
    add_style(styles, "type", palette.type_name);
    add_style(styles, "constructor", palette.type_name);
    add_style(styles, "module", palette.type_name);
    add_style(styles, "namespace", palette.type_name);
    add_style(styles, "variable", palette.variable);
    add_style(styles, "label", palette.variable);
    add_style(styles, "property", palette.property);
    add_style(styles, "operator", palette.operator);
    add_style(styles, "punctuation", palette.punctuation);
    add_style(styles, "attribute", palette.attribute);
    add_style(styles, "decorator", palette.attribute);
    add_style(styles, "tag", palette.tag);
    add_style(styles, "invalid", palette.invalid);
}

fn add_subcapture_styles(styles: &mut BTreeMap<String, SyntaxStyle>, palette: Palette) {
    add_styled(
        styles,
        "comment.documentation",
        SyntaxStyle {
            foreground: Some(palette.comment),
            italic: true,
            ..SyntaxStyle::default()
        },
    );
    add_styled(
        styles,
        "function.macro",
        SyntaxStyle {
            foreground: Some(palette.function),
            italic: true,
            ..SyntaxStyle::default()
        },
    );
    for name in [
        "keyword.control",
        "keyword.function",
        "keyword.import",
        "keyword.return",
        "keyword.storage",
    ] {
        add_styled(
            styles,
            name,
            SyntaxStyle {
                foreground: Some(palette.keyword),
                bold: true,
                ..SyntaxStyle::default()
            },
        );
    }
}

fn add_styled(styles: &mut BTreeMap<String, SyntaxStyle>, name: &str, style: SyntaxStyle) {
    styles.insert(name.to_string(), style);
}

fn add_style(styles: &mut BTreeMap<String, SyntaxStyle>, name: &str, color: Color) {
    styles.insert(name.to_string(), SyntaxStyle::with_foreground(color));
}

pub(super) fn one_dark_palette() -> Palette {
    Palette {
        comment: rgb(92, 99, 112),
        keyword: rgb(198, 120, 221),
        string: rgb(152, 195, 121),
        number: rgb(209, 154, 102),
        constant: rgb(209, 154, 102),
        function: rgb(97, 175, 239),
        type_name: rgb(229, 192, 123),
        variable: rgb(224, 226, 234),
        property: rgb(86, 182, 194),
        operator: rgb(86, 182, 194),
        punctuation: rgb(171, 178, 191),
        attribute: rgb(229, 192, 123),
        tag: rgb(224, 108, 117),
        invalid: rgb(224, 108, 117),
    }
}

pub(super) fn gruvbox_light_palette() -> Palette {
    Palette {
        comment: rgb(146, 131, 116),
        keyword: rgb(157, 0, 6),
        string: rgb(121, 116, 14),
        number: rgb(143, 63, 113),
        constant: rgb(143, 63, 113),
        function: rgb(181, 118, 20),
        type_name: rgb(7, 102, 120),
        variable: rgb(60, 56, 54),
        property: rgb(66, 123, 88),
        operator: rgb(66, 123, 88),
        punctuation: rgb(124, 111, 100),
        attribute: rgb(181, 118, 20),
        tag: rgb(157, 0, 6),
        invalid: rgb(157, 0, 6),
    }
}
