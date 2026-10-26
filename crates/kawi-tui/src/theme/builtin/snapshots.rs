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
            ("fn", "#fb4934"),
            ("compute", "#b8bb26+bold"),
            ("(", "#fe8019"),
            ("value", "#83a598"),
            (":", "#fe8019"),
            ("u64", "#fabd2f"),
            (")", "#fe8019"),
            ("u64", "#fabd2f"),
            ("{", "#fe8019"),
            ("let", "#fb4934"),
            ("\"ok\"", "#b8bb26+italic"),
            (";", "#fe8019"),
            ("}", "#fe8019"),
        ])
    );
}

#[test]
fn default_theme_snapshot_for_python() {
    let source = "def greet(name: str):\n    return \"hello\"\n";
    assert_eq!(
        styled_tokens(LanguageId::Python, source),
        expected(&[
            ("def", "#fb4934"),
            ("greet", "#b8bb26+bold"),
            ("name", "#83a598"),
            ("str", "#fabd2f"),
            ("return", "#fb4934+bold"),
            ("\"hello\"", "#b8bb26+italic"),
        ])
    );
}

#[test]
fn default_theme_snapshot_for_typescript() {
    let source = "function greet(name: string): number {\n    return 1;\n}\n";
    assert_eq!(
        styled_tokens(LanguageId::TypeScript, source),
        expected(&[
            ("function", "#fb4934"),
            ("greet", "#b8bb26+bold"),
            ("(", "#fe8019"),
            ("name", "#83a598"),
            ("string", "#fabd2f"),
            (")", "#fe8019"),
            ("number", "#fabd2f"),
            ("{", "#fe8019"),
            ("return", "#fb4934+bold"),
            ("1", "#d3869b"),
            (";", "#fe8019"),
            ("}", "#fe8019"),
        ])
    );
}
