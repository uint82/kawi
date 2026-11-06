use std::time::Instant;

use super::*;
use crate::test_support::diagnostic_event;
use kawi_lsp::{
    DiagnosticOrigin, DiagnosticPosition, DiagnosticRange, DiagnosticRelatedInformation,
};
use kawi_task::Severity;

fn lsp_diagnostic(line: u32, message: &str, severity: DiagnosticSeverity) -> Diagnostic {
    Diagnostic {
        range: DiagnosticRange {
            start: DiagnosticPosition { line, character: 0 },
            end: DiagnosticPosition { line, character: 4 },
        },
        severity,
        message: message.to_string(),
        source: Some("rustc".to_string()),
        code: Some("E0308".to_string()),
        code_description: None,
        tags: Vec::new(),
        related: Vec::new(),
    }
}

fn pull_event(
    server: &str,
    identifier: Option<&str>,
    path: &Path,
    diagnostics: Vec<Diagnostic>,
) -> DiagnosticsEvent {
    let mut event = diagnostic_event(server, path, diagnostics);
    event.origin = DiagnosticOrigin::Pull;
    event.identifier = identifier.map(str::to_string);
    event
}

fn task_finding(line: usize) -> task::Finding {
    task::Finding {
        path: PathBuf::from("src/main.rs"),
        line,
        column: 1,
        severity: Severity::Error,
        message: format!("task {line}"),
    }
}

#[test]
fn sources_are_replaced_independently() {
    let path = Path::new("src/main.rs");
    let mut problems = Problems::default();
    problems.push_task("check", task_finding(1));
    problems.replace_lsp_test(
        "rust-analyzer",
        path,
        vec![lsp_diagnostic(2, "lsp", DiagnosticSeverity::Error)],
    );
    assert_eq!(problems.total(), 2);
    problems.replace_lsp_test("rust-analyzer", path, Vec::new());
    assert_eq!(problems.total(), 1);
    assert_eq!(problems.visible(Some(path))[0].source, "task:check");
    problems.clear_source("task:check");
    assert_eq!(problems.total(), 0);
}

#[test]
fn lsp_providers_with_the_same_server_are_replaced_independently() {
    let path = Path::new("src/main.rs");
    let mut problems = Problems::default();
    problems.replace_lsp(diagnostic_event(
        "rust-analyzer",
        path,
        vec![lsp_diagnostic(0, "push", DiagnosticSeverity::Error)],
    ));
    problems.replace_lsp(pull_event(
        "rust-analyzer",
        Some("rust-analyzer"),
        path,
        vec![lsp_diagnostic(1, "pull", DiagnosticSeverity::Error)],
    ));
    assert_eq!(problems.total(), 2);
    problems.replace_lsp(pull_event(
        "rust-analyzer",
        Some("rust-analyzer"),
        path,
        Vec::new(),
    ));
    assert_eq!(problems.total(), 1);
    assert_eq!(problems.visible(None)[0].message, "push");
    problems.clear_lsp_source("rust-analyzer");
    assert_eq!(problems.total(), 0);
}

#[test]
fn push_and_pull_problems_are_distinct_without_a_pull_identifier() {
    let path = Path::new("src/main.rs");
    let mut problems = Problems::default();
    problems.replace_lsp(diagnostic_event(
        "server",
        path,
        vec![lsp_diagnostic(0, "push", DiagnosticSeverity::Error)],
    ));
    problems.replace_lsp(pull_event(
        "server",
        None,
        path,
        vec![lsp_diagnostic(1, "pull", DiagnosticSeverity::Error)],
    ));
    assert_eq!(problems.total(), 2);
    problems.replace_lsp(pull_event("server", None, path, Vec::new()));
    assert_eq!(problems.total(), 1);
    assert_eq!(problems.visible(None)[0].message, "push");
}

#[test]
fn rewrites_and_removes_paths_for_a_renamed_directory() {
    let old = Path::new("/repo/old");
    let new = Path::new("/repo/new");
    let mut problems = Problems::default();
    let mut diagnostic = lsp_diagnostic(1, "error", DiagnosticSeverity::Error);
    diagnostic.related.push(DiagnosticRelatedInformation {
        path: old.join("related.rs"),
        line: 2,
        message: "related".to_string(),
    });
    problems.replace_lsp_test("server", &old.join("a.rs"), vec![diagnostic]);
    problems.rewrite_paths(old, new);
    let visible = problems.visible(Some(&new.join("a.rs")));
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].path, new.join("a.rs"));
    assert_eq!(visible[0].related[0].path, new.join("related.rs"));
    problems.remove_paths(new);
    assert_eq!(problems.total(), 0);
}

#[test]
fn task_sources_can_be_marked_stale_and_refreshed() {
    let mut problems = Problems::default();
    problems.push_task("check", task_finding(1));
    problems.mark_tasks_stale();
    assert!(problems.is_stale("task:check"));
    problems.push_task("check", task_finding(2));
    assert!(!problems.is_stale("task:check"));
    problems.mark_tasks_stale();
    problems.clear_source("task:check");
    assert!(!problems.is_stale("task:check"));
}

#[test]
fn diagnostic_index_counts_lsp_sources_without_double_counting_tasks() {
    let path = Path::new("src/main.rs");
    let mut problems = Problems::default();
    problems.replace_lsp_test(
        "rust-analyzer",
        path,
        vec![lsp_diagnostic(0, "lsp", DiagnosticSeverity::Error)],
    );
    problems.push_task("check", task_finding(1));
    let counts = problems.diagnostic_counts(path);
    assert_eq!(counts.errors, 1);
    assert_eq!(problems.diagnostics_at(path, 0).len(), 2);
    problems.clear_lsp_source("rust-analyzer");
    assert_eq!(problems.diagnostic_counts(path).errors, 0);
    let remaining = problems.diagnostics_at(path, 0);
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].source, "task:check");
}

#[test]
fn diagnostic_index_follows_shifted_lsp_positions() {
    let path = Path::new("src/main.rs");
    let mut problems = Problems::default();
    problems.replace_lsp_test(
        "server",
        path,
        vec![lsp_diagnostic(0, "shifted", DiagnosticSeverity::Error)],
    );
    problems.observe_editor(Some(path), 1, Rope::from_str("one\ntwo\n"));
    problems.observe_editor(Some(path), 2, Rope::from_str("zero\none\ntwo\n"));
    assert!(problems.diagnostics_at(path, 0).is_empty());
    let shifted = problems.diagnostics_at(path, 1);
    assert_eq!(shifted.len(), 1);
    assert_eq!(shifted[0].message, "shifted");
}

#[test]
fn deferred_shift_waits_for_idle() {
    let path = Path::new("src/main.rs");
    let mut problems = Problems::default();
    problems.replace_lsp_test(
        "server",
        path,
        vec![lsp_diagnostic(0, "shifted", DiagnosticSeverity::Error)],
    );
    problems.observe_editor_deferred(Some(path), 1, Rope::from_str("one\ntwo\n"));
    problems.observe_editor_deferred(Some(path), 2, Rope::from_str("zero\none\ntwo\n"));
    assert_eq!(problems.diagnostics_at(path, 0).len(), 1);
    problems.flush_pending_shifts(Instant::now() + SHIFT_DEBOUNCE);
    assert!(problems.diagnostics_at(path, 0).is_empty());
    assert_eq!(problems.diagnostics_at(path, 1).len(), 1);
}

#[test]
fn replacing_lsp_diagnostics_cancels_a_pending_shift() {
    let path = Path::new("src/main.rs");
    let mut problems = Problems::default();
    problems.replace_lsp_test(
        "server",
        path,
        vec![lsp_diagnostic(0, "old", DiagnosticSeverity::Error)],
    );
    problems.observe_editor_deferred(Some(path), 1, Rope::from_str("one\ntwo\n"));
    problems.observe_editor_deferred(Some(path), 2, Rope::from_str("zero\none\ntwo\n"));
    problems.replace_lsp_test(
        "server",
        path,
        vec![lsp_diagnostic(5, "new", DiagnosticSeverity::Error)],
    );
    problems.flush_pending_shifts(Instant::now() + SHIFT_DEBOUNCE);
    assert!(problems.diagnostics_at(path, 0).is_empty());
    assert_eq!(problems.diagnostics_at(path, 5).len(), 1);
}

#[test]
fn visible_problems_sort_by_severity_file_and_line() {
    let mut problems = Problems::default();
    problems.replace_lsp_test(
        "server",
        Path::new("b.rs"),
        vec![
            lsp_diagnostic(1, "warning", DiagnosticSeverity::Warning),
            lsp_diagnostic(2, "error", DiagnosticSeverity::Error),
        ],
    );
    problems.replace_lsp_test(
        "server",
        Path::new("a.rs"),
        vec![lsp_diagnostic(5, "error a", DiagnosticSeverity::Error)],
    );
    let visible = problems.visible(None);
    assert_eq!(visible[0].path, PathBuf::from("a.rs"));
    assert_eq!(visible[1].message, "error");
    assert_eq!(visible[2].message, "warning");
}

#[test]
fn filters_apply_to_navigation() {
    let path = Path::new("src/main.rs");
    let mut problems = Problems::default();
    problems.replace_lsp_test(
        "server",
        path,
        vec![
            lsp_diagnostic(0, "error", DiagnosticSeverity::Error),
            lsp_diagnostic(1, "warning", DiagnosticSeverity::Warning),
        ],
    );
    problems.toggle_current_file();
    problems.cycle_severity();
    let (_, _, problem) = problems.advance(Some(path), true).unwrap();
    assert_eq!(problem.severity, ProblemSeverity::Error);
    problems.cycle_severity();
    assert_eq!(
        problems.advance(Some(path), true).unwrap().2.message,
        "warning"
    );
    problems.cycle_severity();
    assert!(problems.advance(Some(path), true).is_none());
}

#[test]
fn navigation_wraps_and_follows_the_current_entry() {
    let path = Path::new("src/main.rs");
    let mut problems = Problems::default();
    problems.replace_lsp_test(
        "server",
        path,
        vec![
            lsp_diagnostic(0, "first", DiagnosticSeverity::Error),
            lsp_diagnostic(2, "second", DiagnosticSeverity::Error),
        ],
    );
    assert_eq!(problems.advance(Some(path), true).unwrap().0, 0);
    assert_eq!(problems.advance(Some(path), true).unwrap().0, 1);
    assert_eq!(problems.advance(Some(path), true).unwrap().0, 0);
    assert_eq!(problems.advance(Some(path), false).unwrap().0, 1);
}

#[test]
fn edits_shift_lsp_ranges_until_the_next_publish() {
    let path = Path::new("src/main.rs");
    let mut problems = Problems::default();
    problems.replace_lsp_test(
        "server",
        path,
        vec![lsp_diagnostic(2, "stale", DiagnosticSeverity::Error)],
    );
    problems.observe_editor(Some(path), 1, Rope::from_str("one\ntwo\nthree\n"));
    problems.observe_editor(Some(path), 2, Rope::from_str("zero\none\ntwo\nthree\n"));
    let visible = problems.visible(None);
    assert_eq!((visible[0].line, visible[0].column), (4, 1));
}

#[test]
fn deletions_collapse_ranges_until_the_next_publish() {
    let path = Path::new("src/main.rs");
    let mut problems = Problems::default();
    problems.replace_lsp_test(
        "server",
        path,
        vec![lsp_diagnostic(2, "stale", DiagnosticSeverity::Error)],
    );
    problems.observe_editor(Some(path), 1, Rope::from_str("one\ntwo\nthree\n"));
    problems.observe_editor(Some(path), 2, Rope::from_str("one\nthree\n"));
    let visible = problems.visible(None);
    assert_eq!(visible[0].line, 2);
}

#[test]
fn task_problems_do_not_shift() {
    let path = Path::new("src/main.rs");
    let mut problems = Problems::default();
    problems.push_task("check", task_finding(3));
    problems.observe_editor(Some(path), 1, Rope::from_str("one\ntwo\nthree\n"));
    problems.observe_editor(Some(path), 2, Rope::from_str("zero\none\ntwo\nthree\n"));
    assert_eq!(problems.visible(None)[0].line, 3);
}

#[test]
fn source_count_counts_task_and_lsp_sources() {
    let path = Path::new("src/main.rs");
    let mut problems = Problems::default();
    problems.push_task("check", task_finding(1));
    problems.replace_lsp_test(
        "rust-analyzer",
        path,
        vec![lsp_diagnostic(0, "lsp", DiagnosticSeverity::Error)],
    );
    assert_eq!(problems.source_count(), 2);
}
