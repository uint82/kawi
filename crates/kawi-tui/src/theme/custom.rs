use crossterm::style::Color;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

use super::color::parse_color;
use super::{BaseColors, MatchColors, SyntaxColors, Theme, ThemeError};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeFile {
    #[serde(default)]
    code_background: Option<String>,
    base: ColorPair,
    search_match: ColorPair,
    search_current: ColorPair,
    substitute: ColorPair,
    selection: ColorPair,
    status: ColorPair,
    gutter: GutterFile,
    git: GitFile,
    diagnostics: DiagnosticsFile,
    syntax: SyntaxFile,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ColorPair {
    foreground: String,
    background: String,
}

impl ColorPair {
    fn colors(&self, section: &str) -> Result<(Color, Color), ThemeError> {
        Ok((
            parse_color(&format!("{section}.foreground"), &self.foreground)?,
            parse_color(&format!("{section}.background"), &self.background)?,
        ))
    }

    fn match_colors(&self, section: &str) -> Result<MatchColors, ThemeError> {
        let (foreground, background) = self.colors(section)?;
        Ok(MatchColors {
            foreground,
            background,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GutterFile {
    line_number: String,
    non_text: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GitFile {
    add: String,
    change: String,
    delete: String,
    staged_add: String,
    staged_change: String,
    staged_delete: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticsFile {
    error: String,
    warning: String,
    information: String,
    hint: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SyntaxFile {
    comment: String,
    keyword: String,
    string: String,
    number: String,
    constant: String,
    function: String,
    #[serde(rename = "type")]
    type_name: String,
    variable: String,
    property: String,
    operator: String,
    punctuation: String,
    attribute: String,
    tag: String,
    invalid: String,
}

impl Theme {
    pub(super) fn from_file(name: &str, themes_dir: &Path) -> Result<Self, ThemeError> {
        let path = theme_path(name, themes_dir)?;
        let text = fs::read_to_string(&path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                ThemeError::not_found(name)
            } else {
                ThemeError::invalid(format!("{}: {error}", path.display()))
            }
        })?;
        Self::from_toml(name, &text)
    }

    fn from_toml(name: &str, text: &str) -> Result<Self, ThemeError> {
        let file: ThemeFile = toml::from_str(text).map_err(|error| {
            ThemeError::invalid(error.message()).at_line(error_line(&error, text))
        })?;
        let (base_foreground, base_background) = file.base.colors("base")?;
        let line_number = parse_color("gutter.line_number", &file.gutter.line_number)?;
        let non_text = parse_color("gutter.non_text", &file.gutter.non_text)?;
        let code_background = match file.code_background.as_deref() {
            Some(value) => parse_color("code_background", value)?,
            None => base_background,
        };
        Ok(Self {
            name: name.to_string(),
            base: BaseColors {
                foreground: base_foreground,
                background: base_background,
            },
            code_background,
            search_match: file.search_match.match_colors("search_match")?,
            search_current: file.search_current.match_colors("search_current")?,
            substitute: file.substitute.match_colors("substitute")?,
            selection: file.selection.match_colors("selection")?,
            status: file.status.match_colors("status")?,
            line_number,
            non_text,
            git_add: parse_color("git.add", &file.git.add)?,
            git_change: parse_color("git.change", &file.git.change)?,
            git_delete: parse_color("git.delete", &file.git.delete)?,
            git_staged_add: parse_color("git.staged_add", &file.git.staged_add)?,
            git_staged_change: parse_color("git.staged_change", &file.git.staged_change)?,
            git_staged_delete: parse_color("git.staged_delete", &file.git.staged_delete)?,
            diagnostic_error: parse_color("diagnostics.error", &file.diagnostics.error)?,
            diagnostic_warning: parse_color("diagnostics.warning", &file.diagnostics.warning)?,
            diagnostic_information: parse_color(
                "diagnostics.information",
                &file.diagnostics.information,
            )?,
            diagnostic_hint: parse_color("diagnostics.hint", &file.diagnostics.hint)?,
            syntax: SyntaxColors {
                comment: parse_color("syntax.comment", &file.syntax.comment)?,
                keyword: parse_color("syntax.keyword", &file.syntax.keyword)?,
                string: parse_color("syntax.string", &file.syntax.string)?,
                number: parse_color("syntax.number", &file.syntax.number)?,
                constant: parse_color("syntax.constant", &file.syntax.constant)?,
                function: parse_color("syntax.function", &file.syntax.function)?,
                type_name: parse_color("syntax.type", &file.syntax.type_name)?,
                variable: parse_color("syntax.variable", &file.syntax.variable)?,
                property: parse_color("syntax.property", &file.syntax.property)?,
                operator: parse_color("syntax.operator", &file.syntax.operator)?,
                punctuation: parse_color("syntax.punctuation", &file.syntax.punctuation)?,
                attribute: parse_color("syntax.attribute", &file.syntax.attribute)?,
                tag: parse_color("syntax.tag", &file.syntax.tag)?,
                invalid: parse_color("syntax.invalid", &file.syntax.invalid)?,
            },
        })
    }
}

fn theme_path(name: &str, themes_dir: &Path) -> Result<PathBuf, ThemeError> {
    if name.is_empty()
        || !name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(ThemeError::invalid(format!("invalid theme name {name:?}")));
    }
    Ok(themes_dir.join(format!("{name}.toml")))
}

fn error_line(error: &toml::de::Error, text: &str) -> Option<usize> {
    let span = error.span()?;
    let before = text.get(..span.start)?;
    Some(before.bytes().filter(|byte| *byte == b'\n').count() + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_syntax::SyntaxGroup;
    use std::sync::atomic::{AtomicU64, Ordering};

    const SECTIONS: &[(&str, &[&str])] = &[
        ("base", &["foreground", "background"]),
        ("search_match", &["foreground", "background"]),
        ("search_current", &["foreground", "background"]),
        ("substitute", &["foreground", "background"]),
        ("selection", &["foreground", "background"]),
        ("status", &["foreground", "background"]),
        ("gutter", &["line_number", "non_text"]),
        (
            "git",
            &[
                "add",
                "change",
                "delete",
                "staged_add",
                "staged_change",
                "staged_delete",
            ],
        ),
        ("diagnostics", &["error", "warning", "information", "hint"]),
        (
            "syntax",
            &[
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
            ],
        ),
    ];

    fn theme_text() -> String {
        let mut text = String::new();
        for (section, fields) in SECTIONS {
            text.push_str(&format!("[{section}]\n"));
            for field in *fields {
                text.push_str(&format!("{field} = \"#010203\"\n"));
            }
            text.push('\n');
        }
        text
    }

    fn temp_dir(name: &str) -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let dir = std::env::temp_dir().join(format!(
            "kawi_theme_{}_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
            name
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn code_background_defaults_to_the_base_background() {
        let theme = Theme::from_toml("personal", &theme_text()).unwrap();
        assert_eq!(theme.code_background, theme.base.background);
    }

    #[test]
    fn loads_a_custom_code_background() {
        let text = format!("code_background = \"#0a0b0c\"\n{}", theme_text());
        let theme = Theme::from_toml("personal", &text).unwrap();
        assert_eq!(
            theme.code_background,
            Color::Rgb {
                r: 10,
                g: 11,
                b: 12,
            }
        );
    }

    #[test]
    fn loads_a_custom_theme() {
        let dir = temp_dir("load");
        fs::write(dir.join("personal.toml"), theme_text()).unwrap();
        let theme = Theme::load("personal", &dir).unwrap();
        assert_eq!(theme.name, "personal");
        assert_eq!(theme.base.background, Color::Rgb { r: 1, g: 2, b: 3 });
        assert_eq!(
            theme.syntax.color(SyntaxGroup::Keyword),
            Color::Rgb { r: 1, g: 2, b: 3 }
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn theme_files_override_builtins() {
        let dir = temp_dir("override");
        let text = theme_text().replacen("background = \"#010203\"", "background = \"#123456\"", 1);
        fs::write(dir.join("default.toml"), text).unwrap();
        let theme = Theme::load("default", &dir).unwrap();
        assert_eq!(
            theme.base.background,
            Color::Rgb {
                r: 18,
                g: 52,
                b: 86
            }
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn builtins_fallback_when_theme_files_are_missing() {
        let dir = temp_dir("fallback");
        let theme = Theme::load("default", &dir).unwrap();
        assert_eq!(theme.name, "default");
        assert_eq!(
            theme.base.background,
            Color::Rgb {
                r: 20,
                g: 22,
                b: 27
            }
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn builtins_fallback_when_theme_files_are_invalid() {
        let dir = temp_dir("invalid_override");
        fs::write(dir.join("default.toml"), "broken").unwrap();
        let theme = Theme::load("default", &dir).unwrap();
        assert_eq!(theme.name, "default");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_theme_reports_not_found() {
        let dir = temp_dir("missing");
        let error = Theme::load("personal", &dir).unwrap_err();
        assert!(error.is_not_found());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_unsafe_theme_names() {
        let dir = temp_dir("unsafe");
        assert!(Theme::load("../personal", &dir).is_err());
        assert!(Theme::load("personal/evil", &dir).is_err());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_unknown_fields() {
        let text = format!("{}\nextra = 1\n", theme_text());
        let error = Theme::from_toml("personal", &text).unwrap_err();
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn rejects_missing_colors() {
        let text = theme_text().replacen("foreground = \"#010203\"\n", "", 1);
        let error = Theme::from_toml("personal", &text).unwrap_err();
        assert!(error.to_string().contains("missing field"));
        assert!(error.to_string().contains("line"));
    }

    #[test]
    fn rejects_invalid_colors() {
        let text = theme_text().replacen("background = \"#010203\"", "background = \"#zzzzzz\"", 1);
        let error = Theme::from_toml("personal", &text).unwrap_err();
        assert!(error.to_string().contains("base.background"));
    }
}
