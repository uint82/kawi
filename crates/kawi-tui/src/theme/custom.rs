use crossterm::style::Color;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::color::parse_color;
use super::style::{parse_styles, StyleValue};
use super::{BaseColors, MatchColors, Theme, ThemeError};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemePackage {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeFile {
    #[serde(default)]
    package: Option<ThemePackage>,
    #[serde(default)]
    float_background: Option<String>,
    #[serde(default)]
    code_background: Option<String>,
    #[serde(default)]
    special_key: Option<String>,
    base: ColorPair,
    search_match: ColorPair,
    search_current: ColorPair,
    substitute: ColorPair,
    selection: ColorPair,
    status: ColorPair,
    #[serde(default)]
    bracket: Option<BracketFile>,
    gutter: GutterFile,
    git: GitFile,
    diagnostics: DiagnosticsFile,
    #[serde(default)]
    decorations: super::decorations::DecorationsFile,
    #[serde(default)]
    plugin_decorations: super::plugin_decorations::PluginDecorationsFile,
    #[serde(default)]
    statusline: Option<super::statusline::StatusLineFile>,
    #[serde(default)]
    palette: BTreeMap<String, String>,
    #[serde(default)]
    completion: BTreeMap<String, String>,
    syntax: BTreeMap<String, StyleValue>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ColorPair {
    foreground: String,
    background: String,
}

impl ColorPair {
    pub(super) fn colors(&self, section: &str) -> Result<(Color, Color), ThemeError> {
        Ok((
            parse_color(&format!("{section}.foreground"), &self.foreground)?,
            parse_color(&format!("{section}.background"), &self.background)?,
        ))
    }

    pub(super) fn match_colors(&self, section: &str) -> Result<MatchColors, ThemeError> {
        let (foreground, background) = self.colors(section)?;
        Ok(MatchColors {
            foreground,
            background,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BracketFile {
    #[serde(rename = "match")]
    matched: ColorPair,
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

impl Theme {
    pub fn available(themes_dir: &Path) -> Vec<String> {
        let mut names: BTreeSet<String> = Self::names()
            .iter()
            .map(|name| (*name).to_string())
            .collect();
        if let Ok(entries) = fs::read_dir(themes_dir) {
            names.extend(
                entries
                    .flatten()
                    .filter_map(|entry| custom_theme_name(&entry)),
            );
            names.retain(|name| Self::validate(name, themes_dir).is_ok());
        }
        names.into_iter().collect()
    }

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

    pub(super) fn from_toml(name: &str, text: &str) -> Result<Self, ThemeError> {
        let file: ThemeFile = toml::from_str(text).map_err(|error| {
            ThemeError::invalid(error.message()).at_line(error_line(&error, text))
        })?;
        let (base_foreground, base_background) = file.base.colors("base")?;
        let line_number = parse_color("gutter.line_number", &file.gutter.line_number)?;
        let non_text = parse_color("gutter.non_text", &file.gutter.non_text)?;
        let float_background = match file.float_background.as_deref() {
            Some(value) => parse_color("float_background", value)?,
            None => base_background,
        };
        let code_background = match file.code_background.as_deref() {
            Some(value) => parse_color("code_background", value)?,
            None => float_background,
        };
        let special_key = match file.special_key.as_deref() {
            Some(value) => parse_color("special_key", value)?,
            None => non_text,
        };
        let selection = file.selection.match_colors("selection")?;
        let palette = super::style::parse_palette(file.palette)?;
        let bracket_match = match file.bracket.as_ref() {
            Some(bracket) => bracket.matched.match_colors("bracket.match")?,
            None => selection.clone(),
        };
        let syntax = parse_styles(file.syntax, &palette)?;
        let completion =
            super::completion::parse(file.completion, base_foreground, non_text, &syntax)?;
        Ok(Self {
            name: name.to_string(),
            package: file.package,
            base: BaseColors {
                foreground: base_foreground,
                background: base_background,
            },
            float_background,
            code_background,
            search_match: file.search_match.match_colors("search_match")?,
            search_current: file.search_current.match_colors("search_current")?,
            substitute: file.substitute.match_colors("substitute")?,
            selection,
            bracket_match,
            status: file.status.match_colors("status")?,
            line_number,
            non_text,
            special_key,
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
            completion,
            decorations: super::decorations::parse(Some(file.decorations))?,
            plugin_decorations: super::plugin_decorations::parse(
                file.plugin_decorations,
                &palette,
            )?,
            statusline: super::statusline::parse(file.statusline)?,
            syntax,
        })
    }
}

fn theme_path(name: &str, themes_dir: &Path) -> Result<PathBuf, ThemeError> {
    if !valid_theme_name(name) {
        return Err(ThemeError::invalid(format!("invalid theme name {name:?}")));
    }
    let file = themes_dir.join(format!("{name}.toml"));
    if file.is_file() {
        return Ok(file);
    }
    let package = themes_dir.join(name).join("theme.toml");
    if package.is_file() {
        return Ok(package);
    }
    Ok(file)
}

fn custom_theme_name(entry: &fs::DirEntry) -> Option<String> {
    let path = entry.path();
    if path.is_file() && path.extension().and_then(|value| value.to_str()) == Some("toml") {
        let name = path.file_stem()?.to_str()?;
        return valid_theme_name(name).then(|| name.to_string());
    }
    if path.is_dir() && path.join("theme.toml").is_file() {
        let name = path.file_name()?.to_str()?;
        return valid_theme_name(name).then(|| name.to_string());
    }
    None
}

fn valid_theme_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

fn error_line(error: &toml::de::Error, text: &str) -> Option<usize> {
    let span = error.span()?;
    let before = text.get(..span.start)?;
    Some(before.bytes().filter(|byte| *byte == b'\n').count() + 1)
}

#[cfg(test)]
mod tests;
