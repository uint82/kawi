use std::path::{Path, PathBuf};

use kawi_git::{BranchStatus, EntryKind, Status, StatusCode, StatusEntry};
use kawi_lsp::{DiagnosticPosition, DiagnosticRange, DiagnosticSeverity as LspSeverity};
use kawi_tui::frame::{DiagnosticSeverity, ExplorerGit, ExplorerGitKind};

use super::ExplorerMarks;
use crate::app::problems::Problems;

fn status(root: &Path, entries: Vec<StatusEntry>) -> Status {
    Status {
        root: root.to_path_buf(),
        git_dir: root.join(".git"),
        branch: BranchStatus::Named {
            name: "main".to_string(),
            oid: None,
            upstream: None,
        },
        operation: kawi_git::OperationState::None,
        entries,
    }
}

fn diagnostic(severity: LspSeverity) -> kawi_lsp::Diagnostic {
    kawi_lsp::Diagnostic {
        range: DiagnosticRange {
            start: DiagnosticPosition {
                line: 0,
                character: 0,
            },
            end: DiagnosticPosition {
                line: 0,
                character: 4,
            },
        },
        severity,
        message: "problem".to_string(),
        source: None,
        code: None,
        code_description: None,
        tags: Vec::new(),
        related: Vec::new(),
    }
}

fn entry(path: &str, index: StatusCode, worktree: StatusCode) -> StatusEntry {
    StatusEntry {
        path: PathBuf::from(path),
        original_path: None,
        index,
        worktree,
        kind: EntryKind::Ordinary,
    }
}

#[test]
fn git_marks_aggregate_to_parent_directories() {
    let root = Path::new("/repo");
    let status = status(
        root,
        vec![entry(
            "src/main.rs",
            StatusCode::Unmodified,
            StatusCode::Modified,
        )],
    );
    let marks = ExplorerMarks::rebuild(root, &Problems::default(), Some(&status));
    assert_eq!(
        marks.git(&root.join("src/main.rs"), false),
        Some(ExplorerGit {
            code: 'M',
            kind: ExplorerGitKind::Change,
        })
    );
    assert_eq!(
        marks.git(&root.join("src"), true),
        Some(ExplorerGit {
            code: 'M',
            kind: ExplorerGitKind::Change,
        })
    );
    assert!(marks.git(root, true).is_none());
}

#[test]
fn diagnostics_aggregate_to_parent_directories_by_severity() {
    let root = Path::new("/repo");
    let mut problems = Problems::default();
    problems.replace_lsp(
        "fake",
        &root.join("src/main.rs"),
        vec![
            diagnostic(LspSeverity::Warning),
            diagnostic(LspSeverity::Error),
        ],
    );
    let marks = ExplorerMarks::rebuild(root, &problems, None);
    assert_eq!(
        marks.diagnostic(&root.join("src/main.rs"), false),
        Some(DiagnosticSeverity::Error)
    );
    assert_eq!(
        marks.diagnostic(&root.join("src"), true),
        Some(DiagnosticSeverity::Error)
    );
    assert!(marks.diagnostic(&root.join("docs"), true).is_none());
    assert!(marks.diagnostic(root, true).is_none());
}

#[test]
fn conflicts_take_priority_over_other_directory_changes() {
    let root = Path::new("/repo");
    let mut conflict = entry(
        "src/conflict.rs",
        StatusCode::Unmerged,
        StatusCode::Unmerged,
    );
    conflict.kind = EntryKind::Unmerged;
    let status = status(
        root,
        vec![
            entry("src/main.rs", StatusCode::Unmodified, StatusCode::Modified),
            conflict,
        ],
    );
    let marks = ExplorerMarks::rebuild(root, &Problems::default(), Some(&status));
    assert_eq!(
        marks.git(&root.join("src"), true),
        Some(ExplorerGit {
            code: '!',
            kind: ExplorerGitKind::Conflict,
        })
    );
}
