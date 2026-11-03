use kawi_lsp::{
    CursorPosition, Diagnostic, DiagnosticPosition, DiagnosticSeverity, DiagnosticsEvent,
    INITIAL_DOCUMENT_VERSION,
};
use kawi_tui::frame::{
    DiagnosticSeverity as UiSeverity, DiagnosticSpan, DiagnosticTag as UiDiagnosticTag,
};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct DiagnosticProvider {
    server: String,
    identifier: Option<String>,
}

#[derive(Default)]
pub(super) struct DiagnosticState {
    version: i32,
    providers: BTreeMap<DiagnosticProvider, Vec<Diagnostic>>,
    items: Vec<Diagnostic>,
}

impl DiagnosticState {
    pub(super) fn reset(&mut self) {
        self.version = INITIAL_DOCUMENT_VERSION;
        self.clear();
    }

    pub(super) fn advance(&mut self) {
        self.version = self.version.saturating_add(1);
        self.clear();
    }

    pub(super) fn apply(&mut self, event: DiagnosticsEvent, path: &Path) -> bool {
        if event.path != path {
            return false;
        }
        if event.version.is_some_and(|version| version != self.version) {
            return false;
        }
        let provider = DiagnosticProvider {
            server: event.server,
            identifier: event.identifier,
        };
        self.providers.insert(provider, event.diagnostics);
        self.rebuild();
        true
    }

    pub(super) fn remove_server(&mut self, server: &str) {
        let before = self.providers.len();
        self.providers
            .retain(|provider, _| provider.server != server);
        if self.providers.len() != before {
            self.rebuild();
        }
    }

    fn clear(&mut self) {
        self.providers.clear();
        self.items.clear();
    }

    fn rebuild(&mut self) {
        self.items.clear();
        self.items
            .extend(self.providers.values().flatten().cloned());
        self.items
            .sort_by_key(|diagnostic| (diagnostic.range.start, diagnostic.range.end));
    }

    pub(super) fn has_diagnostics(&self) -> bool {
        !self.items.is_empty()
    }

    pub(super) fn severity_at(&self, row: usize) -> Option<DiagnosticSeverity> {
        let row = u32::try_from(row).unwrap_or(u32::MAX);
        self.items
            .iter()
            .filter(|diagnostic| diagnostic.range.start.line == row)
            .map(|diagnostic| diagnostic.severity)
            .min()
    }

    pub(super) fn spans(&self, window: &[String], rows: &[usize]) -> Vec<DiagnosticSpan> {
        let mut spans = Vec::new();
        for diagnostic in &self.items {
            collect_spans(&mut spans, diagnostic, window, rows);
        }
        spans
    }

    pub(super) fn in_range(&self, start: CursorPosition, end: CursorPosition) -> Vec<Diagnostic> {
        let start = position(start);
        let end = position(end);
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        self.items
            .iter()
            .filter(|diagnostic| diagnostic.range.end >= start && diagnostic.range.start <= end)
            .cloned()
            .collect()
    }
}

fn position(value: CursorPosition) -> DiagnosticPosition {
    DiagnosticPosition {
        line: value.line,
        character: value.column,
    }
}

fn collect_spans(
    spans: &mut Vec<DiagnosticSpan>,
    diagnostic: &Diagnostic,
    window: &[String],
    rows: &[usize],
) {
    let start_line = usize::try_from(diagnostic.range.start.line).unwrap_or(usize::MAX);
    let end_line = usize::try_from(diagnostic.range.end.line).unwrap_or(usize::MAX);
    for (text, row) in window.iter().zip(rows.iter().copied()) {
        if row < start_line || row > end_line {
            continue;
        }
        let line_len = text.chars().count();
        let Some((start, end)) = columns(diagnostic, row, line_len) else {
            continue;
        };
        spans.push(DiagnosticSpan {
            row,
            start,
            end,
            severity: ui_severity(diagnostic.severity),
            tag: diagnostic_tag(&diagnostic.tags),
        });
    }
}

fn columns(diagnostic: &Diagnostic, row: usize, line_len: usize) -> Option<(usize, usize)> {
    let start_line = usize::try_from(diagnostic.range.start.line).unwrap_or(usize::MAX);
    let end_line = usize::try_from(diagnostic.range.end.line).unwrap_or(usize::MAX);
    let mut start = if row == start_line {
        usize::try_from(diagnostic.range.start.character).unwrap_or(usize::MAX)
    } else {
        0
    }
    .min(line_len);
    let mut end = if row == end_line {
        usize::try_from(diagnostic.range.end.character).unwrap_or(usize::MAX)
    } else {
        line_len
    }
    .min(line_len);
    if start == end && start_line == end_line && line_len > 0 {
        if start == line_len {
            start = start.saturating_sub(1);
        } else {
            end = start.saturating_add(1);
        }
    }
    (end > start).then_some((start, end))
}

fn diagnostic_tag(tags: &[kawi_lsp::DiagnosticTag]) -> Option<UiDiagnosticTag> {
    if tags.contains(&kawi_lsp::DiagnosticTag::Deprecated) {
        return Some(UiDiagnosticTag::Deprecated);
    }
    tags.contains(&kawi_lsp::DiagnosticTag::Unnecessary)
        .then_some(UiDiagnosticTag::Unnecessary)
}

pub(super) fn ui_severity(severity: DiagnosticSeverity) -> UiSeverity {
    match severity {
        DiagnosticSeverity::Error => UiSeverity::Error,
        DiagnosticSeverity::Warning => UiSeverity::Warning,
        DiagnosticSeverity::Information => UiSeverity::Information,
        DiagnosticSeverity::Hint => UiSeverity::Hint,
    }
}

#[cfg(test)]
mod tests;
