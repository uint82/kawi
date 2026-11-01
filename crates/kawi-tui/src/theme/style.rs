use crossterm::style::Color;
use kawi_syntax::{registry, SyntaxHighlight};
use serde::Deserialize;
use std::collections::BTreeMap;

use super::color::parse_color;
use super::ThemeError;

const REQUIRED_BASE_NAMES: &[&str] = &[
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
];

const ALIASES: &[(&str, &str)] = &[
    ("boolean", "constant"),
    ("character", "string"),
    ("constructor", "type"),
    ("decorator", "attribute"),
    ("escape", "string"),
    ("label", "variable"),
    ("module", "type"),
    ("namespace", "type"),
    ("special", "constant"),
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SyntaxStyle {
    pub foreground: Option<Color>,
    pub background: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl SyntaxStyle {
    pub fn with_foreground(foreground: Color) -> Self {
        Self {
            foreground: Some(foreground),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxStyles {
    entries: Vec<Option<SyntaxStyle>>,
}

impl SyntaxStyles {
    pub(crate) fn from_keyed(styles: BTreeMap<String, SyntaxStyle>) -> Self {
        let entries = registry()
            .names()
            .iter()
            .map(|name| resolve_style(&styles, name))
            .collect();
        Self { entries }
    }

    pub fn style(&self, highlight: SyntaxHighlight) -> Option<SyntaxStyle> {
        self.entries
            .get(usize::from(highlight.0))
            .copied()
            .flatten()
    }

    pub fn named(&self, name: &str) -> Option<SyntaxStyle> {
        registry()
            .index_of(name)
            .and_then(|highlight| self.style(highlight))
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum StyleValue {
    Color(String),
    Table(StyleTable),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StyleTable {
    fg: Option<String>,
    bg: Option<String>,
    #[serde(default)]
    bold: bool,
    #[serde(default)]
    italic: bool,
    #[serde(default)]
    underline: bool,
}

pub(super) fn parse_styles(
    values: BTreeMap<String, StyleValue>,
    palette: BTreeMap<String, Color>,
) -> Result<SyntaxStyles, ThemeError> {
    let mut styles = parse_style_map(values, &palette)?;
    validate_base_styles(&styles)?;
    add_aliases(&mut styles);
    Ok(SyntaxStyles::from_keyed(styles))
}

pub(super) fn parse_palette(
    palette: BTreeMap<String, String>,
) -> Result<BTreeMap<String, Color>, ThemeError> {
    palette
        .into_iter()
        .map(|(name, value)| {
            let color = parse_color(&format!("palette.{name}"), &value)?;
            Ok((name, color))
        })
        .collect()
}

fn parse_style_map(
    values: BTreeMap<String, StyleValue>,
    palette: &BTreeMap<String, Color>,
) -> Result<BTreeMap<String, SyntaxStyle>, ThemeError> {
    values
        .into_iter()
        .map(|(name, value)| {
            let style = parse_style(&name, value, palette)?;
            Ok((name, style))
        })
        .collect()
}

fn validate_base_styles(styles: &BTreeMap<String, SyntaxStyle>) -> Result<(), ThemeError> {
    for name in REQUIRED_BASE_NAMES {
        if !styles.contains_key(*name) {
            return Err(ThemeError::invalid(format!("missing syntax.{name}")));
        }
    }
    Ok(())
}

fn parse_style(
    name: &str,
    value: StyleValue,
    palette: &BTreeMap<String, Color>,
) -> Result<SyntaxStyle, ThemeError> {
    match value {
        StyleValue::Color(color) => Ok(SyntaxStyle::with_foreground(resolve_color(
            &format!("syntax.{name}"),
            &color,
            palette,
        )?)),
        StyleValue::Table(table) => Ok(SyntaxStyle {
            foreground: table
                .fg
                .as_deref()
                .map(|value| resolve_color(&format!("syntax.{name}.fg"), value, palette))
                .transpose()?,
            background: table
                .bg
                .as_deref()
                .map(|value| resolve_color(&format!("syntax.{name}.bg"), value, palette))
                .transpose()?,
            bold: table.bold,
            italic: table.italic,
            underline: table.underline,
        }),
    }
}

pub(super) fn resolve_color(
    field: &str,
    value: &str,
    palette: &BTreeMap<String, Color>,
) -> Result<Color, ThemeError> {
    if let Some(color) = palette.get(value) {
        return Ok(*color);
    }
    parse_color(field, value)
}

fn add_aliases(styles: &mut BTreeMap<String, SyntaxStyle>) {
    for (alias, source) in ALIASES {
        if styles.contains_key(*alias) {
            continue;
        }
        if let Some(style) = styles.get(*source).copied() {
            styles.insert((*alias).to_string(), style);
        }
    }
}

fn resolve_style(styles: &BTreeMap<String, SyntaxStyle>, name: &str) -> Option<SyntaxStyle> {
    let mut candidate = name;
    loop {
        if let Some(style) = styles.get(candidate) {
            return Some(*style);
        }
        let (parent, _) = candidate.rsplit_once('.')?;
        candidate = parent;
    }
}

#[cfg(test)]
mod tests;
