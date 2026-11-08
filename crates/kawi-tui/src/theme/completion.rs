use crossterm::style::Color;
use kawi_completion::CompletionKind;
use std::collections::BTreeMap;

use super::color::parse_color;
use super::{SyntaxStyles, ThemeError};

const KIND_COUNT: usize = CompletionKind::ALL.len();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionTheme {
    colors: [Color; KIND_COUNT],
    matched: Color,
}

impl CompletionTheme {
    pub fn color(&self, kind: CompletionKind) -> Color {
        self.colors[kind.index()]
    }

    pub fn matched(&self) -> Color {
        self.matched
    }
}

pub(super) fn parse(
    overrides: BTreeMap<String, String>,
    muted: Color,
    matched: Color,
    syntax: &SyntaxStyles,
) -> Result<CompletionTheme, ThemeError> {
    let mut theme = CompletionTheme {
        colors: default_colors(muted, syntax),
        matched,
    };
    for (key, value) in overrides {
        let color = parse_color(&format!("completion.{key}"), &value)?;
        if key == "match" {
            theme.matched = color;
            continue;
        }
        let kind = CompletionKind::from_key(&key).ok_or_else(|| {
            ThemeError::invalid(format!("completion section has unknown kind {key:?}"))
        })?;
        theme.colors[kind.index()] = color;
    }
    Ok(theme)
}

fn default_colors(muted: Color, syntax: &SyntaxStyles) -> [Color; KIND_COUNT] {
    let mut colors = [muted; KIND_COUNT];
    for kind in CompletionKind::ALL {
        colors[kind.index()] = syntax_color(syntax, kind).unwrap_or(muted);
    }
    colors
}

fn syntax_color(syntax: &SyntaxStyles, kind: CompletionKind) -> Option<Color> {
    let name = match kind {
        CompletionKind::Method | CompletionKind::Function => "function",
        CompletionKind::Constructor => "constructor",
        CompletionKind::Field | CompletionKind::Property => "property",
        CompletionKind::Variable => "attribute",
        CompletionKind::Class
        | CompletionKind::Interface
        | CompletionKind::Struct
        | CompletionKind::Enum
        | CompletionKind::TypeParameter => "type",
        CompletionKind::Module => "special",
        CompletionKind::Unit
        | CompletionKind::Value
        | CompletionKind::Constant
        | CompletionKind::EnumMember => "constant",
        CompletionKind::Keyword => "keyword",
        CompletionKind::Operator => "special",
        CompletionKind::Snippet => "string",
        CompletionKind::Color => "special",
        CompletionKind::Text => "comment",
        CompletionKind::File
        | CompletionKind::Reference
        | CompletionKind::Folder
        | CompletionKind::Event => return None,
    };
    syntax.named(name).and_then(|style| style.foreground)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use std::collections::BTreeMap;
    use std::path::Path;

    fn theme_with(completion: &str) -> Result<Theme, ThemeError> {
        let text = format!(
            "{}\n[completion]\n{completion}\n",
            include_str!("../../themes/default.toml")
        );
        Theme::from_toml("test", &text)
    }

    #[test]
    fn defaults_follow_syntax_colors() {
        let theme = Theme::default();
        let syntax = |name| {
            theme
                .syntax
                .named(name)
                .and_then(|style| style.foreground)
                .expect("syntax color")
        };
        assert_eq!(
            theme.completion.color(CompletionKind::Function),
            syntax("function")
        );
        assert_eq!(
            theme.completion.color(CompletionKind::Method),
            syntax("function")
        );
        assert_eq!(
            theme.completion.color(CompletionKind::Class),
            syntax("type")
        );
        assert_eq!(
            theme.completion.color(CompletionKind::Keyword),
            syntax("keyword")
        );
        assert_eq!(
            theme.completion.color(CompletionKind::Text),
            syntax("comment")
        );
        assert_eq!(
            theme.completion.color(CompletionKind::Variable),
            syntax("attribute")
        );
        assert_eq!(
            theme.completion.color(CompletionKind::Module),
            syntax("special")
        );
        assert_eq!(theme.completion.color(CompletionKind::File), theme.non_text);
        assert_eq!(theme.completion.matched(), theme.search_match.background);
        for kind in CompletionKind::ALL {
            assert_ne!(theme.completion.color(kind), theme.base.foreground);
        }
    }

    #[test]
    fn every_builtin_theme_colors_every_kind() {
        for name in Theme::names() {
            let theme = Theme::named(name).expect("built-in theme");
            for kind in CompletionKind::ALL {
                assert_ne!(
                    theme.completion.color(kind),
                    theme.base.foreground,
                    "{name} left {kind:?} uncolored"
                );
            }
        }
    }

    #[test]
    fn missing_syntax_colors_fall_back_to_muted() {
        let muted = Color::Rgb { r: 1, g: 2, b: 3 };
        let colors = default_colors(muted, &SyntaxStyles::from_keyed(BTreeMap::new()));
        assert!(colors.iter().all(|color| *color == muted));
    }

    #[test]
    fn overrides_replace_individual_kinds() {
        let theme =
            theme_with("function = \"#010203\"\ntype_parameter = \"#040506\"\nmatch = \"#070809\"")
                .unwrap();
        assert_eq!(
            theme.completion.color(CompletionKind::Function),
            Color::Rgb { r: 1, g: 2, b: 3 }
        );
        assert_eq!(
            theme.completion.color(CompletionKind::TypeParameter),
            Color::Rgb { r: 4, g: 5, b: 6 }
        );
        assert_ne!(
            theme.completion.color(CompletionKind::Keyword),
            Color::Rgb { r: 1, g: 2, b: 3 }
        );
        assert_eq!(theme.completion.matched(), Color::Rgb { r: 7, g: 8, b: 9 });
    }

    #[test]
    fn unknown_kinds_are_rejected() {
        let error = theme_with("nope = \"#010203\"").unwrap_err();
        assert!(error.to_string().contains("unknown kind"));
    }

    #[test]
    fn invalid_colors_are_rejected() {
        let error = theme_with("function = \"not-a-color\"").unwrap_err();
        assert!(error.to_string().contains("completion.function"));
    }

    #[test]
    fn the_themes_directory_is_not_required() {
        let missing = Path::new("/nonexistent/kawi/themes");
        let theme = Theme::load("default", missing).expect("built-in theme");
        assert_ne!(
            theme.completion.color(CompletionKind::Text),
            theme.base.foreground
        );
    }
}
