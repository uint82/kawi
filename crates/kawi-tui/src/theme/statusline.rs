use serde::Deserialize;

use crate::frame::DiagnosticSeverity;
use crate::statusbar::{ModeKind, SegmentStyle};

use super::custom::ColorPair;
use super::{MatchColors, Theme, ThemeError};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatusLineTheme {
    pub mode_normal: Option<MatchColors>,
    pub mode_insert: Option<MatchColors>,
    pub mode_replace: Option<MatchColors>,
    pub mode_visual: Option<MatchColors>,
    pub mode_command: Option<MatchColors>,
    pub mode_search: Option<MatchColors>,
    pub diagnostics_error: Option<MatchColors>,
    pub diagnostics_warning: Option<MatchColors>,
    pub diagnostics_information: Option<MatchColors>,
    pub diagnostics_hint: Option<MatchColors>,
    pub modified: Option<MatchColors>,
    pub readonly: Option<MatchColors>,
    pub git: Option<MatchColors>,
    pub lsp: Option<MatchColors>,
    pub selection: Option<MatchColors>,
    pub macro_recording: Option<MatchColors>,
    pub filetype: Option<MatchColors>,
    pub indent: Option<MatchColors>,
    pub encoding: Option<MatchColors>,
    pub eol: Option<MatchColors>,
    pub percent: Option<MatchColors>,
    pub position: Option<MatchColors>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(super) struct StatusLineFile {
    #[serde(default)]
    mode: StatusLineModeFile,
    #[serde(default)]
    diagnostics: StatusLineDiagnosticFile,
    #[serde(default)]
    modified: Option<ColorPair>,
    #[serde(default)]
    readonly: Option<ColorPair>,
    #[serde(default)]
    git: Option<ColorPair>,
    #[serde(default)]
    lsp: Option<ColorPair>,
    #[serde(default)]
    selection: Option<ColorPair>,
    #[serde(default)]
    macro_recording: Option<ColorPair>,
    #[serde(default)]
    filetype: Option<ColorPair>,
    #[serde(default)]
    indent: Option<ColorPair>,
    #[serde(default)]
    encoding: Option<ColorPair>,
    #[serde(default)]
    eol: Option<ColorPair>,
    #[serde(default)]
    percent: Option<ColorPair>,
    #[serde(default)]
    position: Option<ColorPair>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct StatusLineModeFile {
    #[serde(default)]
    normal: Option<ColorPair>,
    #[serde(default)]
    insert: Option<ColorPair>,
    #[serde(default)]
    replace: Option<ColorPair>,
    #[serde(default)]
    visual: Option<ColorPair>,
    #[serde(default)]
    command: Option<ColorPair>,
    #[serde(default)]
    search: Option<ColorPair>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct StatusLineDiagnosticFile {
    #[serde(default)]
    error: Option<ColorPair>,
    #[serde(default)]
    warning: Option<ColorPair>,
    #[serde(default)]
    information: Option<ColorPair>,
    #[serde(default)]
    hint: Option<ColorPair>,
}

pub(super) fn parse(file: Option<StatusLineFile>) -> Result<StatusLineTheme, ThemeError> {
    let Some(file) = file else {
        return Ok(StatusLineTheme::default());
    };
    Ok(StatusLineTheme {
        mode_normal: pair(file.mode.normal, "statusline.mode.normal")?,
        mode_insert: pair(file.mode.insert, "statusline.mode.insert")?,
        mode_replace: pair(file.mode.replace, "statusline.mode.replace")?,
        mode_visual: pair(file.mode.visual, "statusline.mode.visual")?,
        mode_command: pair(file.mode.command, "statusline.mode.command")?,
        mode_search: pair(file.mode.search, "statusline.mode.search")?,
        diagnostics_error: pair(file.diagnostics.error, "statusline.diagnostics.error")?,
        diagnostics_warning: pair(file.diagnostics.warning, "statusline.diagnostics.warning")?,
        diagnostics_information: pair(
            file.diagnostics.information,
            "statusline.diagnostics.information",
        )?,
        diagnostics_hint: pair(file.diagnostics.hint, "statusline.diagnostics.hint")?,
        modified: pair(file.modified, "statusline.modified")?,
        readonly: pair(file.readonly, "statusline.readonly")?,
        git: pair(file.git, "statusline.git")?,
        lsp: pair(file.lsp, "statusline.lsp")?,
        selection: pair(file.selection, "statusline.selection")?,
        macro_recording: pair(file.macro_recording, "statusline.macro_recording")?,
        filetype: pair(file.filetype, "statusline.filetype")?,
        indent: pair(file.indent, "statusline.indent")?,
        encoding: pair(file.encoding, "statusline.encoding")?,
        eol: pair(file.eol, "statusline.eol")?,
        percent: pair(file.percent, "statusline.percent")?,
        position: pair(file.position, "statusline.position")?,
    })
}

fn pair(pair: Option<ColorPair>, key: &str) -> Result<Option<MatchColors>, ThemeError> {
    pair.map(|pair| pair.match_colors(key)).transpose()
}

impl StatusLineTheme {
    fn mode(&self, kind: ModeKind) -> Option<MatchColors> {
        match kind {
            ModeKind::Normal => self.mode_normal.clone(),
            ModeKind::Insert => self.mode_insert.clone(),
            ModeKind::Replace => self.mode_replace.clone(),
            ModeKind::Visual => self.mode_visual.clone(),
            ModeKind::Command => self.mode_command.clone(),
            ModeKind::Search => self.mode_search.clone(),
        }
    }

    fn diagnostic(&self, severity: DiagnosticSeverity) -> Option<MatchColors> {
        match severity {
            DiagnosticSeverity::Error => self.diagnostics_error.clone(),
            DiagnosticSeverity::Warning => self.diagnostics_warning.clone(),
            DiagnosticSeverity::Information => self.diagnostics_information.clone(),
            DiagnosticSeverity::Hint => self.diagnostics_hint.clone(),
        }
    }
}

impl Theme {
    pub fn statusline_colors(&self, style: SegmentStyle, active: bool) -> MatchColors {
        if !active {
            return MatchColors {
                foreground: self.line_number,
                background: self.base.background,
            };
        }
        let base = self.status.clone();
        let tinted = |color| MatchColors {
            foreground: color,
            background: base.background,
        };
        match style {
            SegmentStyle::Default => base,
            SegmentStyle::Mode(kind) => self
                .statusline
                .mode(kind)
                .unwrap_or_else(|| default_mode_colors(self, kind)),
            SegmentStyle::Modified => self
                .statusline
                .modified
                .clone()
                .unwrap_or_else(|| tinted(self.diagnostic_warning)),
            SegmentStyle::ReadOnly => self
                .statusline
                .readonly
                .clone()
                .unwrap_or_else(|| tinted(self.diagnostic_information)),
            SegmentStyle::Diagnostic(severity) => self
                .statusline
                .diagnostic(severity)
                .unwrap_or_else(|| tinted(self.diagnostic_color(severity))),
            SegmentStyle::Git => self
                .statusline
                .git
                .clone()
                .unwrap_or_else(|| tinted(self.git_change)),
            SegmentStyle::Lsp => self
                .statusline
                .lsp
                .clone()
                .unwrap_or_else(|| tinted(self.diagnostic_information)),
            SegmentStyle::Selection => self
                .statusline
                .selection
                .clone()
                .unwrap_or_else(|| tinted(self.search_match.background)),
            SegmentStyle::Macro => self
                .statusline
                .macro_recording
                .clone()
                .unwrap_or_else(|| tinted(self.diagnostic_error)),
            SegmentStyle::FileType => self.statusline.filetype.clone().unwrap_or(base),
            SegmentStyle::Indent => self.statusline.indent.clone().unwrap_or(base),
            SegmentStyle::Encoding => self.statusline.encoding.clone().unwrap_or(base),
            SegmentStyle::Eol => self.statusline.eol.clone().unwrap_or(base),
            SegmentStyle::Percent => self.statusline.percent.clone().unwrap_or(base),
            SegmentStyle::Position => self.statusline.position.clone().unwrap_or(base),
        }
    }
}

fn default_mode_colors(theme: &Theme, kind: ModeKind) -> MatchColors {
    let (foreground, background) = match kind {
        ModeKind::Normal => (theme.base.background, theme.diagnostic_information),
        ModeKind::Insert => (theme.base.background, theme.git_add),
        ModeKind::Replace => (theme.base.background, theme.diagnostic_warning),
        ModeKind::Visual => (theme.status.foreground, theme.selection.background),
        ModeKind::Command | ModeKind::Search => (theme.status.foreground, theme.status.background),
    };
    MatchColors {
        foreground,
        background,
    }
}

#[cfg(test)]
mod tests;
