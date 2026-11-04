use anyhow::Result;
use crossterm::style::Color;

use kawi_syntax::SyntaxHighlight;

use crate::paint;
use crate::surface::Surface;
use crate::theme::Theme;

use super::highlight::write_segment;

#[derive(Debug, Clone, Copy, Default)]
struct MarkdownRenderStyle {
    foreground: Option<Color>,
    background: Option<Color>,
    bold: bool,
    italic: bool,
    underline: bool,
}

fn markdown_style(
    style: crate::markdown::MarkdownStyle,
    theme: &Theme,
) -> Option<MarkdownRenderStyle> {
    use crate::markdown::MarkdownStyle;
    let syntax = |name: &str| theme.syntax.named(name).and_then(|style| style.foreground);
    match style {
        MarkdownStyle::Normal => None,
        MarkdownStyle::Strong | MarkdownStyle::Heading => Some(MarkdownRenderStyle {
            foreground: syntax("keyword"),
            bold: true,
            ..MarkdownRenderStyle::default()
        }),
        MarkdownStyle::Emphasis => Some(MarkdownRenderStyle {
            foreground: syntax("function"),
            italic: true,
            ..MarkdownRenderStyle::default()
        }),
        MarkdownStyle::Code => Some(MarkdownRenderStyle {
            foreground: syntax("string"),
            ..MarkdownRenderStyle::default()
        }),
        MarkdownStyle::Quote => Some(MarkdownRenderStyle {
            foreground: syntax("comment"),
            ..MarkdownRenderStyle::default()
        }),
        MarkdownStyle::Syntax(highlight) => Some(theme.syntax.style(highlight).map_or_else(
            MarkdownRenderStyle::default,
            |style| MarkdownRenderStyle {
                foreground: style.foreground,
                background: style.background,
                bold: style.bold,
                italic: style.italic,
                underline: style.underline,
            },
        )),
    }
}

pub(super) fn paint_syntax(
    out: &mut Surface,
    text: &str,
    highlight: SyntaxHighlight,
    theme: &Theme,
    background: Color,
) -> Result<()> {
    apply_syntax_style(out, highlight, theme, background);
    write_segment(out, text)?;
    out.reset_attributes();
    paint::apply_colors(out, theme.base.foreground, background);
    Ok(())
}

pub(super) fn apply_syntax_style(
    out: &mut Surface,
    highlight: SyntaxHighlight,
    theme: &Theme,
    background: Color,
) {
    let style = theme.syntax.style(highlight).unwrap_or_default();
    out.reset_attributes();
    paint::apply_colors(
        out,
        style.foreground.unwrap_or(theme.base.foreground),
        style.background.unwrap_or(background),
    );
    apply_attributes(out, style.bold, style.italic, style.underline);
}

pub(super) fn apply_attributes(out: &mut Surface, bold: bool, italic: bool, underline: bool) {
    out.set_bold(bold);
    out.set_italic(italic);
    if underline {
        out.set_plain_underline();
    }
}

pub(super) fn apply_markdown_style(
    out: &mut Surface,
    style: crate::markdown::MarkdownStyle,
    background: Color,
    theme: &Theme,
) -> Result<()> {
    out.reset_attributes();
    let style = markdown_style(style, theme).unwrap_or_default();
    let background = style.background.unwrap_or(background);
    paint::apply_colors(
        out,
        style.foreground.unwrap_or(theme.base.foreground),
        background,
    );
    apply_attributes(out, style.bold, style.italic, style.underline);
    Ok(())
}

#[cfg(test)]
mod tests {
    use crossterm::style::Color;

    use super::*;

    fn highlight_named(name: &str) -> SyntaxHighlight {
        kawi_syntax::registry()
            .index_of(name)
            .expect("registered capture")
    }

    #[test]
    fn syntax_attributes_reach_overlay_style() {
        use crate::theme::{SyntaxStyle, SyntaxStyles};
        use std::collections::BTreeMap;
        let mut theme = Theme::default();
        let mut styles = BTreeMap::new();
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
                .expect("base color");
            styles.insert(name.to_string(), SyntaxStyle::with_foreground(color));
        }
        styles.insert(
            "comment".to_string(),
            SyntaxStyle {
                background: Some(Color::Blue),
                bold: true,
                italic: true,
                underline: true,
                ..theme.syntax.named("comment").expect("comment")
            },
        );
        theme.syntax = SyntaxStyles::from_keyed(styles);
        let style = markdown_style(
            crate::markdown::MarkdownStyle::Syntax(highlight_named("comment")),
            &theme,
        )
        .expect("syntax style");
        assert_eq!(style.background, Some(Color::Blue));
        assert!(style.bold);
        assert!(style.italic);
        assert!(style.underline);
    }

    #[test]
    fn theme_change_recolors_overlay_syntax() {
        let default = Theme::named("default").expect("default theme");
        let gruvbox = Theme::named("one-dark").expect("one dark theme");
        let style = crate::markdown::MarkdownStyle::Syntax(highlight_named("keyword"));
        let default_color = markdown_style(style, &default)
            .and_then(|style| style.foreground)
            .expect("default color");
        let gruvbox_color = markdown_style(style, &gruvbox)
            .and_then(|style| style.foreground)
            .expect("gruvbox color");
        assert_ne!(default_color, gruvbox_color);
    }
}
