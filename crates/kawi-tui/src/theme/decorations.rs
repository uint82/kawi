use serde::Deserialize;

use crate::frame::DecorationStyle;

use super::custom::ColorPair;
use super::{MatchColors, Theme, ThemeError};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DecorationTheme {
    pub accent: Option<MatchColors>,
    pub info: Option<MatchColors>,
    pub success: Option<MatchColors>,
    pub warning: Option<MatchColors>,
    pub error: Option<MatchColors>,
    pub muted: Option<MatchColors>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(super) struct DecorationsFile {
    #[serde(default)]
    accent: Option<ColorPair>,
    #[serde(default)]
    info: Option<ColorPair>,
    #[serde(default)]
    success: Option<ColorPair>,
    #[serde(default)]
    warning: Option<ColorPair>,
    #[serde(default)]
    error: Option<ColorPair>,
    #[serde(default)]
    muted: Option<ColorPair>,
}

pub(super) fn parse(file: Option<DecorationsFile>) -> Result<DecorationTheme, ThemeError> {
    let Some(file) = file else {
        return Ok(DecorationTheme::default());
    };
    Ok(DecorationTheme {
        accent: pair(file.accent, "decorations.accent")?,
        info: pair(file.info, "decorations.info")?,
        success: pair(file.success, "decorations.success")?,
        warning: pair(file.warning, "decorations.warning")?,
        error: pair(file.error, "decorations.error")?,
        muted: pair(file.muted, "decorations.muted")?,
    })
}

fn pair(pair: Option<ColorPair>, key: &str) -> Result<Option<MatchColors>, ThemeError> {
    pair.map(|pair| pair.match_colors(key)).transpose()
}

impl Theme {
    pub fn decoration_colors(&self, style: DecorationStyle) -> MatchColors {
        if let Some(colors) = self.decorations.select(style) {
            return colors.clone();
        }
        match style {
            DecorationStyle::Accent => self.search_match.clone(),
            DecorationStyle::Info => self.filled(self.diagnostic_information),
            DecorationStyle::Success => self.filled(self.git_add),
            DecorationStyle::Warning => self.filled(self.diagnostic_warning),
            DecorationStyle::Error => self.filled(self.diagnostic_error),
            DecorationStyle::Muted => MatchColors {
                foreground: self.non_text,
                background: self.status.background,
            },
        }
    }

    pub fn decoration_color(&self, style: DecorationStyle) -> crossterm::style::Color {
        self.decoration_colors(style).background
    }

    fn filled(&self, background: crossterm::style::Color) -> MatchColors {
        MatchColors {
            foreground: self.base.background,
            background,
        }
    }
}

impl DecorationTheme {
    fn select(&self, style: DecorationStyle) -> Option<&MatchColors> {
        match style {
            DecorationStyle::Accent => self.accent.as_ref(),
            DecorationStyle::Info => self.info.as_ref(),
            DecorationStyle::Success => self.success.as_ref(),
            DecorationStyle::Warning => self.warning.as_ref(),
            DecorationStyle::Error => self.error.as_ref(),
            DecorationStyle::Muted => self.muted.as_ref(),
        }
    }
}
