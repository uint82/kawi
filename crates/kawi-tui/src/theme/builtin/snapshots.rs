use crossterm::style::Color;

use super::*;
use crate::theme::SyntaxStyle;
use kawi_syntax::{highlight_snippet, LanguageId};

fn style_label(style: SyntaxStyle) -> String {
    let color = style.foreground.unwrap_or(Color::Reset);
    let (r, g, b) = match color {
        Color::Rgb { r, g, b } => (r, g, b),
        _ => (0, 0, 0),
    };
    let mut label = format!("#{r:02x}{g:02x}{b:02x}");
    if style.bold {
        label.push_str("+bold");
    }
    if style.italic {
        label.push_str("+italic");
    }
    if style.underline {
        label.push_str("+underline");
    }
    label
}

fn styled_tokens(language: LanguageId, source: &str) -> Vec<(String, String)> {
    let theme = Theme::default();
    highlight_snippet(language, source)
        .into_iter()
        .map(|span| {
            let text = source[span.start..span.end].to_string();
            let style = theme.syntax.style(span.highlight).unwrap_or_default();
            (text, style_label(style))
        })
        .collect()
}

fn expected(tokens: &[(&str, &str)]) -> Vec<(String, String)> {
    tokens
        .iter()
        .map(|(text, style)| ((*text).to_string(), (*style).to_string()))
        .collect()
}

#[test]
fn default_theme_snapshot_for_rust() {
    let source = "fn compute(value: u64) -> u64 {\n    let text = \"ok\";\n    value\n}\n";
    assert_eq!(
        styled_tokens(LanguageId::Rust, source),
        expected(&[
            ("fn", "#ff7e87"),
            ("compute", "#6ed0f0+bold"),
            ("(", "#8c9e9b"),
            ("value", "#f0a6b8"),
            (":", "#8c9e9b"),
            ("u64", "#f2c27b"),
            (")", "#8c9e9b"),
            ("u64", "#f2c27b"),
            ("{", "#8c9e9b"),
            ("let", "#ff7e87"),
            ("\"ok\"", "#a8d98a+italic"),
            (";", "#8c9e9b"),
            ("}", "#8c9e9b"),
        ])
    );
}

#[test]
fn default_theme_snapshot_for_python() {
    let source = "def greet(name: str):\n    return \"hello\"\n";
    assert_eq!(
        styled_tokens(LanguageId::Python, source),
        expected(&[
            ("def", "#ff7e87"),
            ("greet", "#6ed0f0+bold"),
            ("name", "#f0a6b8"),
            ("str", "#f2c27b"),
            ("return", "#ff7e87+bold"),
            ("\"hello\"", "#a8d98a+italic"),
        ])
    );
}

#[test]
fn default_theme_snapshot_for_typescript() {
    let source = "function greet(name: string): number {\n    return 1;\n}\n";
    assert_eq!(
        styled_tokens(LanguageId::TypeScript, source),
        expected(&[
            ("function", "#ff7e87"),
            ("greet", "#6ed0f0+bold"),
            ("(", "#8c9e9b"),
            ("name", "#f0a6b8"),
            (":", "#8c9e9b"),
            ("string", "#f2c27b"),
            (")", "#8c9e9b"),
            (":", "#8c9e9b"),
            ("number", "#f2c27b"),
            ("{", "#8c9e9b"),
            ("return", "#ff7e87+bold"),
            ("1", "#c9a7ff"),
            (";", "#8c9e9b"),
            ("}", "#8c9e9b"),
        ])
    );
}
