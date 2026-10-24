use kawi_lsp::{
    CursorPosition, Diagnostic, DiagnosticPosition, DiagnosticSeverity, DiagnosticsEvent,
    INITIAL_DOCUMENT_VERSION,
};
use kawi_tui::frame::{DiagnosticSeverity as UiSeverity, DiagnosticSpan};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Default)]
pub(super) struct DiagnosticState {
    version: i32,
    servers: BTreeMap<String, Vec<Diagnostic>>,
    items: Vec<Diagnostic>,
}

pub(super) struct Navigation {
    pub(super) row: usize,
    pub(super) column: usize,
    pub(super) index: usize,
    pub(super) total: usize,
    pub(super) severity: DiagnosticSeverity,
    pub(super) message: String,
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
        self.servers.insert(event.server, event.diagnostics);
        self.rebuild();
        true
    }

    pub(super) fn remove_server(&mut self, server: &str) {
        if self.servers.remove(server).is_some() {
            self.rebuild();
        }
    }

    fn clear(&mut self) {
        self.servers.clear();
        self.items.clear();
    }

    fn rebuild(&mut self) {
        self.items.clear();
        self.items.extend(self.servers.values().flatten().cloned());
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

    pub(super) fn spans(&self, window: &[String], top: usize) -> Vec<DiagnosticSpan> {
        let mut spans = Vec::new();
        for diagnostic in &self.items {
            collect_spans(&mut spans, diagnostic, window, top);
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

    pub(super) fn navigate(&self, row: usize, column: usize, forward: bool) -> Option<Navigation> {
        if self.items.is_empty() {
            return None;
        }
        let cursor = DiagnosticPosition {
            line: u32::try_from(row).unwrap_or(u32::MAX),
            character: u32::try_from(column).unwrap_or(u32::MAX),
        };
        let index = if forward {
            self.items
                .iter()
                .position(|diagnostic| diagnostic.range.start > cursor)
                .unwrap_or(0)
        } else {
            self.items
                .iter()
                .rposition(|diagnostic| diagnostic.range.start < cursor)
                .unwrap_or(self.items.len().saturating_sub(1))
        };
        let diagnostic = self.items.get(index)?;
        Some(Navigation {
            row: usize::try_from(diagnostic.range.start.line).unwrap_or(usize::MAX),
            column: usize::try_from(diagnostic.range.start.character).unwrap_or(usize::MAX),
            index: index.saturating_add(1),
            total: self.items.len(),
            severity: diagnostic.severity,
            message: diagnostic.message.clone(),
        })
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
    top: usize,
) {
    if window.is_empty() {
        return;
    }
    let start_line = usize::try_from(diagnostic.range.start.line).unwrap_or(usize::MAX);
    let end_line = usize::try_from(diagnostic.range.end.line).unwrap_or(usize::MAX);
    let first = start_line.max(top);
    let last = end_line.min(top.saturating_add(window.len()).saturating_sub(1));
    if first > last {
        return;
    }
    for row in first..=last {
        let Some(text) = window.get(row.saturating_sub(top)) else {
            continue;
        };
        let line_len = text.chars().count();
        let Some((start, end)) = columns(diagnostic, row, line_len) else {
            continue;
        };
        spans.push(DiagnosticSpan {
            row,
            start,
            end,
            severity: ui_severity(diagnostic.severity),
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

fn ui_severity(severity: DiagnosticSeverity) -> UiSeverity {
    match severity {
        DiagnosticSeverity::Error => UiSeverity::Error,
        DiagnosticSeverity::Warning => UiSeverity::Warning,
        DiagnosticSeverity::Information => UiSeverity::Information,
        DiagnosticSeverity::Hint => UiSeverity::Hint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_lsp::DiagnosticRange;

    fn diagnostic(start: (u32, u32), end: (u32, u32), message: &str) -> Diagnostic {
        Diagnostic {
            range: DiagnosticRange {
                start: DiagnosticPosition {
                    line: start.0,
                    character: start.1,
                },
                end: DiagnosticPosition {
                    line: end.0,
                    character: end.1,
                },
            },
            severity: DiagnosticSeverity::Error,
            message: message.to_string(),
            source: None,
            code: None,
            tags: Vec::new(),
            related: Vec::new(),
        }
    }

    fn event(path: &Path, version: Option<i32>, diagnostics: Vec<Diagnostic>) -> DiagnosticsEvent {
        event_from("fake", path, version, diagnostics)
    }

    fn event_from(
        server: &str,
        path: &Path,
        version: Option<i32>,
        diagnostics: Vec<Diagnostic>,
    ) -> DiagnosticsEvent {
        DiagnosticsEvent {
            server: server.to_string(),
            path: path.to_path_buf(),
            version,
            diagnostics,
        }
    }

    #[test]
    fn stale_and_foreign_diagnostics_are_ignored() {
        let path = Path::new("/tmp/lsp_diagnostic_state");
        let mut state = DiagnosticState::default();
        state.reset();
        assert!(state.apply(
            event(path, Some(1), vec![diagnostic((0, 0), (0, 1), "one")]),
            path
        ));
        assert!(state.has_diagnostics());
        assert!(!state.apply(
            event(path, Some(0), vec![diagnostic((0, 0), (0, 1), "stale")]),
            path
        ));
        assert!(!state.apply(
            event(
                Path::new("/tmp/other"),
                Some(1),
                vec![diagnostic((0, 0), (0, 1), "other")]
            ),
            path
        ));
        state.advance();
        assert!(!state.has_diagnostics());
        assert!(state.apply(
            event(path, Some(2), vec![diagnostic((0, 0), (0, 1), "two")]),
            path
        ));
    }

    #[test]
    fn server_diagnostics_are_merged_and_removable() {
        let path = Path::new("/tmp/lsp_diagnostic_merge");
        let mut state = DiagnosticState::default();
        state.reset();
        assert!(state.apply(
            event_from(
                "alpha",
                path,
                Some(1),
                vec![diagnostic((0, 0), (0, 1), "a")]
            ),
            path
        ));
        assert!(state.apply(
            event_from("beta", path, Some(1), vec![diagnostic((1, 0), (1, 1), "b")]),
            path
        ));
        assert_eq!(state.items.len(), 2);
        assert_eq!(state.severity_at(0), Some(DiagnosticSeverity::Error));
        assert_eq!(state.severity_at(1), Some(DiagnosticSeverity::Error));
        state.remove_server("alpha");
        assert_eq!(state.items.len(), 1);
        assert_eq!(state.severity_at(0), None);
        assert_eq!(state.severity_at(1), Some(DiagnosticSeverity::Error));
    }

    #[test]
    fn spans_clip_to_visible_lines_and_extend_zero_width_diagnostics() {
        let state = DiagnosticState {
            version: 1,
            items: vec![
                diagnostic((0, 1), (0, 1), "point"),
                diagnostic((0, 2), (2, 1), "multi"),
            ],
            ..DiagnosticState::default()
        };
        let window = ["abcd".to_string(), String::new(), "ef".to_string()];
        let spans = state.spans(&window, 0);
        assert!(spans.contains(&DiagnosticSpan {
            row: 0,
            start: 1,
            end: 2,
            severity: UiSeverity::Error,
        }));
        assert!(spans.contains(&DiagnosticSpan {
            row: 0,
            start: 2,
            end: 4,
            severity: UiSeverity::Error,
        }));
        assert!(spans.contains(&DiagnosticSpan {
            row: 2,
            start: 0,
            end: 1,
            severity: UiSeverity::Error,
        }));
    }

    #[test]
    fn navigation_wraps_in_both_directions() {
        let state = DiagnosticState {
            version: 1,
            items: vec![
                diagnostic((0, 0), (0, 1), "first"),
                diagnostic((2, 0), (2, 1), "second"),
            ],
            ..DiagnosticState::default()
        };
        let next = state.navigate(1, 0, true).expect("next");
        assert_eq!((next.row, next.index, next.total), (2, 2, 2));
        let wrapped = state.navigate(3, 0, true).expect("wrapped next");
        assert_eq!(wrapped.row, 0);
        let previous = state.navigate(2, 0, false).expect("previous");
        assert_eq!(previous.row, 0);
        let wrapped_previous = state.navigate(0, 0, false).expect("wrapped previous");
        assert_eq!(wrapped_previous.row, 2);
    }

    #[test]
    fn severity_at_prefers_the_most_urgent_diagnostic() {
        let mut state = DiagnosticState {
            version: 1,
            items: vec![
                diagnostic((3, 0), (3, 1), "error"),
                diagnostic((3, 2), (3, 3), "warning"),
            ],
            ..DiagnosticState::default()
        };
        state.items[1].severity = DiagnosticSeverity::Warning;
        assert_eq!(state.severity_at(3), Some(DiagnosticSeverity::Error));
        assert_eq!(state.severity_at(4), None);
    }
}
