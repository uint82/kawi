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
        code_description: None,
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
    let spans = state.spans(&window, &[0, 1, 2]);
    assert!(spans.contains(&DiagnosticSpan {
        row: 0,
        start: 1,
        end: 2,
        severity: UiSeverity::Error,
        tag: None,
    }));
    assert!(spans.contains(&DiagnosticSpan {
        row: 0,
        start: 2,
        end: 4,
        severity: UiSeverity::Error,
        tag: None,
    }));
    assert!(spans.contains(&DiagnosticSpan {
        row: 2,
        start: 0,
        end: 1,
        severity: UiSeverity::Error,
        tag: None,
    }));
}

#[test]
fn tags_map_to_diagnostic_decorations() {
    let mut unnecessary = diagnostic((0, 0), (0, 1), "unused");
    unnecessary.tags = vec![kawi_lsp::DiagnosticTag::Unnecessary];
    let mut deprecated = diagnostic((1, 0), (1, 1), "deprecated");
    deprecated.tags = vec![
        kawi_lsp::DiagnosticTag::Unnecessary,
        kawi_lsp::DiagnosticTag::Deprecated,
    ];
    let state = DiagnosticState {
        version: 1,
        items: vec![unnecessary, deprecated],
        ..DiagnosticState::default()
    };
    let spans = state.spans(&["one".to_string(), "two".to_string()], &[0, 1]);
    assert_eq!(spans[0].tag, Some(UiDiagnosticTag::Unnecessary));
    assert_eq!(spans[1].tag, Some(UiDiagnosticTag::Deprecated));
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
