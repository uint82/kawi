use kawi_tui::frame::{DiagnosticSpan, DiagnosticUnderline, VirtualText};
use std::path::Path;

use super::diagnostic_deferral::diagnostics_deferred;
use super::diagnostic_index::DiagnosticCounts;
use super::lsp_diagnostics::ui_severity;
use super::problems::{ProblemSeverity, Problems};
use super::App;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum VirtualTextMode {
    Off,
    #[default]
    CursorLine,
    All,
}

impl VirtualTextMode {
    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name {
            "off" => Some(Self::Off),
            "cursor_line" => Some(Self::CursorLine),
            "all" => Some(Self::All),
            _ => None,
        }
    }

    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::CursorLine => "cursor_line",
            Self::All => "all",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum SeverityFilter {
    Error,
    #[default]
    Warning,
    Information,
    Hint,
}

impl SeverityFilter {
    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name {
            "error" => Some(Self::Error),
            "warning" => Some(Self::Warning),
            "information" => Some(Self::Information),
            "hint" => Some(Self::Hint),
            _ => None,
        }
    }

    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Information => "information",
            Self::Hint => "hint",
        }
    }

    pub(super) fn includes(self, severity: ProblemSeverity) -> bool {
        severity <= self.into()
    }
}

impl From<SeverityFilter> for ProblemSeverity {
    fn from(filter: SeverityFilter) -> Self {
        match filter {
            SeverityFilter::Error => Self::Error,
            SeverityFilter::Warning => Self::Warning,
            SeverityFilter::Information => Self::Information,
            SeverityFilter::Hint => Self::Hint,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum SourceDisplay {
    Always,
    #[default]
    IfMany,
}

impl SourceDisplay {
    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name {
            "always" => Some(Self::Always),
            "if_many" => Some(Self::IfMany),
            _ => None,
        }
    }

    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::IfMany => "if_many",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum CountsMode {
    #[default]
    ErrorsWarnings,
    All,
}

impl CountsMode {
    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name {
            "errors_warnings" => Some(Self::ErrorsWarnings),
            "all" => Some(Self::All),
            _ => None,
        }
    }

    pub(super) fn name(self) -> &'static str {
        match self {
            Self::ErrorsWarnings => "errors_warnings",
            Self::All => "all",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DiagnosticOptions {
    pub(super) virtual_text: VirtualTextMode,
    pub(super) virtual_severity: SeverityFilter,
    pub(super) source: SourceDisplay,
    pub(super) counts: CountsMode,
    pub(super) workspace_counts: bool,
    pub(super) underline: DiagnosticUnderline,
}

impl Default for DiagnosticOptions {
    fn default() -> Self {
        Self {
            virtual_text: VirtualTextMode::All,
            virtual_severity: SeverityFilter::Hint,
            source: SourceDisplay::IfMany,
            counts: CountsMode::ErrorsWarnings,
            workspace_counts: false,
            underline: DiagnosticUnderline::Curly,
        }
    }
}

impl DiagnosticOptions {
    #[cfg(test)]
    pub(super) fn counts_text(
        self,
        buffer: DiagnosticCounts,
        workspace: DiagnosticCounts,
    ) -> Option<String> {
        let buffer = self.format_counts(buffer);
        let workspace = self
            .workspace_counts
            .then(|| self.format_counts(workspace))
            .flatten();
        match (buffer, workspace) {
            (None, None) => None,
            (Some(buffer), None) => Some(buffer),
            (None, Some(workspace)) => Some(format!("workspace {workspace}")),
            (Some(buffer), Some(workspace)) => Some(format!("{buffer}  workspace {workspace}")),
        }
    }

    pub(super) fn format_counts(self, counts: DiagnosticCounts) -> Option<String> {
        let mut parts = Vec::new();
        if counts.errors > 0 {
            parts.push(format!("E{}", counts.errors));
        }
        if counts.warnings > 0 {
            parts.push(format!("W{}", counts.warnings));
        }
        if self.counts == CountsMode::All {
            if counts.information > 0 {
                parts.push(format!("I{}", counts.information));
            }
            if counts.hints > 0 {
                parts.push(format!("H{}", counts.hints));
            }
        }
        (!parts.is_empty()).then(|| parts.join(" "))
    }
}

pub(super) struct VirtualTextRequest<'a> {
    pub(super) path: Option<&'a Path>,
    pub(super) rows: &'a [usize],
    pub(super) cursor_row: usize,
    pub(super) cursor_column: usize,
}

impl App {
    pub(super) fn diagnostics_deferred(&self) -> bool {
        diagnostics_deferred(self.editor.mode())
    }

    pub(super) fn diagnostics_visible(&self) -> bool {
        !self.diagnostics_deferred()
    }

    pub(super) fn visible_diagnostic_spans(
        &self,
        lines: &[String],
        rows: &[usize],
    ) -> Vec<DiagnosticSpan> {
        if !self.diagnostics_visible() {
            return Vec::new();
        }
        self.lsp.diagnostic_spans(lines, rows)
    }

    pub(super) fn virtual_texts(
        &self,
        request: VirtualTextRequest<'_>,
    ) -> Vec<Option<VirtualText>> {
        if !self.diagnostics_visible() {
            return request.rows.iter().map(|_| None).collect();
        }
        build_virtual_texts(&self.problems, &self.diagnostic_options, request)
    }
}

fn build_virtual_texts(
    problems: &Problems,
    options: &DiagnosticOptions,
    request: VirtualTextRequest<'_>,
) -> Vec<Option<VirtualText>> {
    let Some(path) = request.path else {
        return Vec::new();
    };
    if options.virtual_text == VirtualTextMode::Off {
        return Vec::new();
    }
    request
        .rows
        .iter()
        .copied()
        .map(|row| {
            if options.virtual_text == VirtualTextMode::CursorLine && row != request.cursor_row {
                return None;
            }
            let column = (row == request.cursor_row).then_some(request.cursor_column);
            problems
                .primary_diagnostic(path, row, column)
                .filter(|problem| options.virtual_severity.includes(problem.severity))
                .and_then(|problem| {
                    let text = virtual_text_line(&problem.message);
                    (!text.is_empty()).then(|| {
                        VirtualText::diagnostic(text, ui_severity(problem.severity.into()))
                    })
                })
        })
        .collect()
}

fn virtual_text_line(message: &str) -> String {
    message
        .lines()
        .next()
        .unwrap_or_default()
        .chars()
        .map(|character| {
            if character == '\t' || character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod visibility_tests;
