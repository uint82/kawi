use super::*;
use kawi_lsp::{Diagnostic, DiagnosticPosition, DiagnosticRange, DiagnosticSeverity};
use std::path::PathBuf;

fn diagnostic(line: u32, severity: DiagnosticSeverity, message: &str) -> Diagnostic {
    Diagnostic {
        range: DiagnosticRange {
            start: DiagnosticPosition { line, character: 0 },
            end: DiagnosticPosition { line, character: 4 },
        },
        severity,
        message: message.to_string(),
        source: Some("fake".to_string()),
        code: None,
        code_description: None,
        tags: Vec::new(),
        related: Vec::new(),
    }
}

fn problems(diagnostics: Vec<Diagnostic>) -> (Problems, PathBuf) {
    let path = PathBuf::from("/tmp/virtual_text.rs");
    let mut problems = Problems::default();
    problems.replace_lsp("fake", &path, diagnostics);
    (problems, path)
}

fn request<'a>(
    path: &'a Path,
    top: usize,
    height: usize,
    cursor_row: usize,
    suppress: bool,
) -> VirtualTextRequest<'a> {
    let rows: &'static [usize] = Box::leak(
        (top..top.saturating_add(height))
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    );
    VirtualTextRequest {
        path: Some(path),
        rows,
        cursor_row,
        cursor_column: 0,
        suppress,
    }
}

#[test]
fn counts_hide_clean_buffers_and_optional_severities() {
    let options = DiagnosticOptions::default();
    assert_eq!(
        options.counts_text(DiagnosticCounts::default(), DiagnosticCounts::default()),
        None
    );
    assert_eq!(
        options.format_counts(DiagnosticCounts {
            errors: 1,
            warnings: 2,
            information: 3,
            hints: 4,
        }),
        Some("E1 W2".to_string())
    );
    let all = DiagnosticOptions {
        counts: CountsMode::All,
        ..options
    };
    assert_eq!(
        all.format_counts(DiagnosticCounts {
            errors: 1,
            warnings: 2,
            information: 3,
            hints: 4,
        }),
        Some("E1 W2 I3 H4".to_string())
    );
}

#[test]
fn workspace_counts_are_opt_in() {
    let buffer = DiagnosticCounts {
        errors: 1,
        ..DiagnosticCounts::default()
    };
    let workspace = DiagnosticCounts {
        errors: 3,
        ..DiagnosticCounts::default()
    };
    let mut options = DiagnosticOptions::default();
    assert_eq!(
        options.counts_text(buffer, workspace),
        Some("E1".to_string())
    );
    options.workspace_counts = true;
    assert_eq!(
        options.counts_text(buffer, workspace),
        Some("E1  workspace E3".to_string())
    );
}

#[test]
fn status_counts_read_the_cached_index() {
    let path = PathBuf::from("/tmp/status_counts.rs");
    let mut app = App::new(
        kawi_buffer::Buffer::from_string("value\n"),
        crate::keymap::Flavor::Vim,
    );
    app.problems.replace_lsp(
        "fake",
        &path,
        vec![
            diagnostic(0, DiagnosticSeverity::Error, "error"),
            diagnostic(1, DiagnosticSeverity::Warning, "warning"),
            diagnostic(2, DiagnosticSeverity::Information, "info"),
        ],
    );
    assert_eq!(
        app.diagnostic_status_text(Some(&path)),
        Some("E1 W1".to_string())
    );
    app.diagnostic_options.workspace_counts = true;
    assert_eq!(
        app.diagnostic_status_text(Some(&path)),
        Some("E1 W1  workspace E1 W1".to_string())
    );
    app.problems.clear_lsp_source("fake");
    assert_eq!(app.diagnostic_status_text(Some(&path)), None);
}

#[test]
fn virtual_text_modes_follow_the_cursor_and_severity_filter() {
    let (problems, path) = problems(vec![
        diagnostic(0, DiagnosticSeverity::Error, "first\nsecond"),
        diagnostic(1, DiagnosticSeverity::Warning, "warning"),
        diagnostic(2, DiagnosticSeverity::Information, "info"),
    ]);
    let cursor_line = DiagnosticOptions {
        virtual_text: VirtualTextMode::CursorLine,
        ..DiagnosticOptions::default()
    };
    let texts = build_virtual_texts(&problems, &cursor_line, request(&path, 0, 3, 1, false));
    assert_eq!(texts.len(), 3);
    assert!(texts[0].is_none());
    assert_eq!(texts[1].as_ref().unwrap().text, "warning");
    assert!(texts[2].is_none());
    let all = DiagnosticOptions::default();
    let texts = build_virtual_texts(&problems, &all, request(&path, 0, 3, 0, false));
    assert_eq!(texts[0].as_ref().unwrap().text, "first");
    assert_eq!(texts[1].as_ref().unwrap().text, "warning");
    assert_eq!(texts[2].as_ref().unwrap().text, "info");
}

#[test]
fn virtual_text_does_not_change_cursor_selection_search_or_horizontal_scroll() {
    let path = PathBuf::from("/tmp/virtual_state.rs");
    let line = format!("{}tail", "x".repeat(120));
    let mut app = App::new(
        kawi_buffer::Buffer::from_string(&format!("{line}\n")),
        crate::keymap::Flavor::Vim,
    );
    app.problems.replace_lsp(
        "fake",
        &path,
        vec![diagnostic(0, DiagnosticSeverity::Error, "message")],
    );
    app.handle_key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('$'),
        crossterm::event::KeyModifiers::NONE,
    ))
    .expect("end");
    app.handle_key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('v'),
        crossterm::event::KeyModifiers::NONE,
    ))
    .expect("visual");
    let before = (
        app.cursor_position(),
        app.editor_selection(),
        app.search_status_line(),
        app.editor.view_leftcol(),
    );
    let texts = app.virtual_texts(VirtualTextRequest {
        path: Some(&path),
        rows: &[0],
        cursor_row: app.editor.row(),
        cursor_column: app.editor.col(),
        suppress: false,
    });
    assert_eq!(texts[0].as_ref().unwrap().text, "message");
    assert_eq!(
        before,
        (
            app.cursor_position(),
            app.editor_selection(),
            app.search_status_line(),
            app.editor.view_leftcol(),
        )
    );
}

#[test]
fn insert_mode_hides_virtual_text_until_the_idle_delay() {
    let mut app = App::new(
        kawi_buffer::Buffer::from_string("value\n"),
        crate::keymap::Flavor::Vim,
    );
    app.handle_key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('i'),
        crossterm::event::KeyModifiers::NONE,
    ))
    .expect("insert");
    assert!(app.virtual_text_suppressed());
    assert!(app.diagnostic_idle_wait_timeout().is_some());
    app.last_input = Instant::now() - DIAGNOSTIC_IDLE_DELAY;
    assert!(!app.virtual_text_suppressed());
    assert!(app.diagnostic_idle_wait_timeout().is_none());
    app.handle_key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
    ))
    .expect("normal");
    assert!(!app.virtual_text_suppressed());
}

#[test]
fn virtual_text_hides_in_insert_mode() {
    let (problems, path) = problems(vec![diagnostic(0, DiagnosticSeverity::Error, "message")]);
    let options = DiagnosticOptions::default();
    assert!(build_virtual_texts(&problems, &options, request(&path, 0, 1, 0, true)).is_empty());
}

#[test]
fn virtual_text_on_other_lines_uses_the_most_severe_diagnostic() {
    let (problems, path) = problems(vec![
        diagnostic(1, DiagnosticSeverity::Warning, "warning"),
        diagnostic(1, DiagnosticSeverity::Error, "error"),
    ]);
    let options = DiagnosticOptions {
        virtual_text: VirtualTextMode::All,
        ..DiagnosticOptions::default()
    };
    let texts = build_virtual_texts(&problems, &options, request(&path, 0, 2, 0, false));
    assert_eq!(texts[1].as_ref().unwrap().text, "error");
}

#[test]
fn virtual_text_first_line_strips_controls() {
    let (problems, path) = problems(vec![diagnostic(
        0,
        DiagnosticSeverity::Error,
        "bad\u{1b}\tmessage\nignored",
    )]);
    let options = DiagnosticOptions::default();
    let texts = build_virtual_texts(&problems, &options, request(&path, 0, 1, 0, false));
    assert_eq!(texts[0].as_ref().unwrap().text, "bad  message");
}

#[test]
fn severity_filters_include_higher_severities() {
    assert!(SeverityFilter::Error.includes(ProblemSeverity::Error));
    assert!(!SeverityFilter::Error.includes(ProblemSeverity::Warning));
    assert!(SeverityFilter::Warning.includes(ProblemSeverity::Error));
    assert!(!SeverityFilter::Warning.includes(ProblemSeverity::Information));
}
