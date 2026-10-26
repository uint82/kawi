use super::*;
use crate::app::problems::ProblemTag;
use std::path::PathBuf;

fn problem(
    source: &str,
    line: usize,
    column: usize,
    end_column: usize,
    severity: ProblemSeverity,
    message: &str,
) -> Problem {
    Problem {
        source: source.to_string(),
        path: PathBuf::from("/tmp/index.rs"),
        line,
        column,
        end_line: line,
        end_column,
        severity,
        message: message.to_string(),
        code: None,
        code_description: None,
        provider: None,
        tags: Vec::<ProblemTag>::new(),
        related: Vec::new(),
    }
}

fn build(problems: &[Problem], ranks: &[(&str, usize)]) -> DiagnosticIndex {
    let ranks = ranks
        .iter()
        .map(|(source, rank)| ((*source).to_string(), *rank))
        .collect();
    DiagnosticIndex::build(
        problems
            .iter()
            .map(|problem| (Path::new("/tmp/index.rs"), problem.source.as_str(), problem)),
        &ranks,
        &BTreeSet::new(),
    )
}

#[test]
fn line_lookup_sorts_by_severity_column_and_source_rank() {
    let problems = [
        problem("lsp:b", 2, 5, 6, ProblemSeverity::Warning, "warning"),
        problem("lsp:a", 2, 8, 9, ProblemSeverity::Error, "late error"),
        problem("lsp:a", 2, 1, 2, ProblemSeverity::Error, "early error"),
    ];
    let index = build(&problems, &[("lsp:a", 0), ("lsp:b", 1)]);
    let messages: Vec<&str> = index
        .at(Path::new("/tmp/index.rs"), 1)
        .iter()
        .map(|problem| problem.message.as_str())
        .collect();
    assert_eq!(messages, ["early error", "late error", "warning"]);
}

#[test]
fn equal_positions_use_source_order() {
    let problems = [
        problem("lsp:b", 1, 1, 2, ProblemSeverity::Error, "b"),
        problem("lsp:a", 1, 1, 2, ProblemSeverity::Error, "a"),
    ];
    let index = build(&problems, &[("lsp:a", 0), ("lsp:b", 1)]);
    let messages: Vec<&str> = index
        .at(Path::new("/tmp/index.rs"), 0)
        .iter()
        .map(|problem| problem.message.as_str())
        .collect();
    assert_eq!(messages, ["a", "b"]);
}

#[test]
fn primary_prefers_the_cursor_column_then_severity_then_earliest() {
    let problems = [
        problem("lsp:a", 2, 1, 2, ProblemSeverity::Error, "first"),
        problem("lsp:a", 2, 6, 8, ProblemSeverity::Warning, "cursor"),
    ];
    let path = Path::new("/tmp/index.rs");
    let index = build(&problems, &[("lsp:a", 0)]);
    assert_eq!(index.primary(path, 1, Some(6)).unwrap().message, "cursor");
    assert_eq!(index.primary(path, 1, Some(0)).unwrap().message, "first");
}

#[test]
fn cursor_overlap_uses_half_open_ranges() {
    let problems = [problem("lsp:a", 1, 2, 3, ProblemSeverity::Error, "single")];
    assert!(overlaps(&problems[0], 0, 1));
    assert!(!overlaps(&problems[0], 0, 2));
    let zero = problem("lsp:a", 1, 4, 4, ProblemSeverity::Warning, "zero");
    assert!(overlaps(&zero, 0, 3));
}

#[test]
fn counts_only_include_lsp_sources() {
    let problems = [
        problem("lsp:rust-analyzer", 1, 1, 2, ProblemSeverity::Error, "lsp"),
        problem("task:check", 1, 1, 2, ProblemSeverity::Error, "task"),
        problem(
            "lsp:rust-analyzer",
            2,
            1,
            2,
            ProblemSeverity::Warning,
            "warn",
        ),
    ];
    let index = build(&problems, &[("lsp:rust-analyzer", 0), ("task:check", 1)]);
    let counts = index.counts(Path::new("/tmp/index.rs"));
    assert_eq!(counts.errors, 1);
    assert_eq!(counts.warnings, 1);
    assert_eq!(index.workspace_counts().errors, 1);
}

#[test]
fn navigation_uses_lsp_diagnostics_only() {
    let problems = [
        problem("lsp:a", 1, 1, 2, ProblemSeverity::Error, "first"),
        problem("task:check", 2, 1, 2, ProblemSeverity::Error, "task"),
        problem("lsp:a", 3, 1, 2, ProblemSeverity::Warning, "last"),
    ];
    let index = build(&problems, &[("lsp:a", 0), ("task:check", 1)]);
    let next = index
        .navigate_lsp(Path::new("/tmp/index.rs"), 0, 0, true)
        .unwrap();
    assert_eq!((next.row, next.index, next.total), (2, 2, 2));
    let wrapped = index
        .navigate_lsp(Path::new("/tmp/index.rs"), 2, 0, true)
        .unwrap();
    assert_eq!((wrapped.row, wrapped.message.as_str()), (0, "first"));
    let previous = index
        .navigate_lsp(Path::new("/tmp/index.rs"), 2, 0, false)
        .unwrap();
    assert_eq!(previous.message, "first");
}
