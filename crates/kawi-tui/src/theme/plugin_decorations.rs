use std::collections::BTreeMap;

use crossterm::style::Color;
use serde::Deserialize;

use crate::frame::PluginFill;

use super::style::resolve_color;
use super::ThemeError;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PluginDecorationOverride {
    pub foreground: Option<Color>,
    pub background: Option<Color>,
    pub fill: Option<PluginFill>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PluginDecorationOverrides {
    groups: BTreeMap<String, BTreeMap<String, PluginDecorationOverride>>,
}

impl PluginDecorationOverrides {
    pub fn get(&self, plugin: &str, group: &str) -> Option<&PluginDecorationOverride> {
        self.groups.get(plugin)?.get(group)
    }
}

#[derive(Deserialize, Default)]
pub(super) struct PluginDecorationsFile {
    #[serde(flatten)]
    plugins: BTreeMap<String, BTreeMap<String, PluginDecorationFile>>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct PluginDecorationFile {
    #[serde(default)]
    foreground: Option<String>,
    #[serde(default)]
    background: Option<String>,
    #[serde(default)]
    fill: Option<PluginFill>,
    #[serde(default)]
    bold: Option<bool>,
    #[serde(default)]
    italic: Option<bool>,
    #[serde(default)]
    underline: Option<bool>,
}

pub(super) fn parse(
    file: PluginDecorationsFile,
    palette: &BTreeMap<String, Color>,
) -> Result<PluginDecorationOverrides, ThemeError> {
    let mut groups = BTreeMap::new();
    for (plugin, entries) in file.plugins {
        let mut overrides = BTreeMap::new();
        for (group, entry) in entries {
            let key = format!("plugin_decorations.{plugin}.{group}");
            overrides.insert(group, parse_override(&key, entry, palette)?);
        }
        groups.insert(plugin, overrides);
    }
    Ok(PluginDecorationOverrides { groups })
}

fn parse_override(
    key: &str,
    entry: PluginDecorationFile,
    palette: &BTreeMap<String, Color>,
) -> Result<PluginDecorationOverride, ThemeError> {
    Ok(PluginDecorationOverride {
        foreground: entry
            .foreground
            .map(|value| resolve_color(&format!("{key}.foreground"), &value, palette))
            .transpose()?,
        background: entry
            .background
            .map(|value| resolve_color(&format!("{key}.background"), &value, palette))
            .transpose()?,
        fill: entry.fill,
        bold: entry.bold,
        italic: entry.italic,
        underline: entry.underline,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::style::Color;

    fn palette() -> BTreeMap<String, Color> {
        BTreeMap::from([("marker".to_string(), Color::Red)])
    }

    #[test]
    fn parses_group_overrides() {
        let file: PluginDecorationsFile = toml::from_str(
            r##"
[markers.todo]
foreground = "marker"
background = "#010203"
fill = "foreground"
bold = true
italic = true
underline = true
"##,
        )
        .expect("overrides");
        let overrides = parse(file, &palette()).expect("parse");
        let value = overrides.get("markers", "todo").expect("group");
        assert_eq!(value.foreground, Some(Color::Red));
        assert_eq!(value.background, Some(Color::Rgb { r: 1, g: 2, b: 3 }));
        assert_eq!(value.fill, Some(PluginFill::Foreground));
        assert_eq!(value.bold, Some(true));
        assert_eq!(value.italic, Some(true));
        assert_eq!(value.underline, Some(true));
    }

    #[test]
    fn rejects_invalid_colors_and_fields() {
        let file: PluginDecorationsFile = toml::from_str(
            r#"
[markers.todo]
foreground = "not-a-color"
"#,
        )
        .expect("overrides");
        assert!(parse(file, &palette()).is_err());
        assert!(toml::from_str::<PluginDecorationsFile>(
            r#"
[markers.todo]
unknown = true
"#
        )
        .is_err());
    }
}
