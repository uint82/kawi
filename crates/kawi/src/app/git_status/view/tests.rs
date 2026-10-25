use super::*;
use kawi_git::{StatusCode, StatusEntry};

fn status(entries: Vec<StatusEntry>) -> Status {
    Status {
        root: Default::default(),
        git_dir: Default::default(),
        branch: kawi_git::BranchStatus::Named {
            name: "main".to_string(),
            oid: None,
            upstream: None,
        },
        operation: kawi_git::OperationState::None,
        entries,
    }
}

fn entry(path: &str, index: StatusCode, worktree: StatusCode, kind: EntryKind) -> StatusEntry {
    StatusEntry {
        path: path.into(),
        original_path: None,
        index,
        worktree,
        kind,
    }
}

#[test]
fn groups_entries_into_sections() {
    let view = GitStatusView::new(
        &status(vec![
            entry(
                "staged.rs",
                StatusCode::Modified,
                StatusCode::Unmodified,
                EntryKind::Ordinary,
            ),
            entry(
                "unstaged.rs",
                StatusCode::Unmodified,
                StatusCode::Modified,
                EntryKind::Ordinary,
            ),
            entry(
                "new.rs",
                StatusCode::Untracked,
                StatusCode::Untracked,
                EntryKind::Untracked,
            ),
            entry(
                "conflict.rs",
                StatusCode::Unmerged,
                StatusCode::Unmerged,
                EntryKind::Unmerged,
            ),
        ]),
        None,
    );
    let lines = view.lines();
    assert!(lines.contains(&"-- Conflicts (1) --".to_string()));
    assert!(lines.contains(&"-- Staged (1) --".to_string()));
    assert!(lines.contains(&"-- Unstaged (1) --".to_string()));
    assert!(lines.contains(&"-- Untracked (1) --".to_string()));
}

#[test]
fn navigation_skips_headers_and_wraps() {
    let mut view = GitStatusView::new(
        &status(vec![
            entry(
                "a.rs",
                StatusCode::Modified,
                StatusCode::Unmodified,
                EntryKind::Ordinary,
            ),
            entry(
                "b.rs",
                StatusCode::Modified,
                StatusCode::Unmodified,
                EntryKind::Ordinary,
            ),
        ]),
        None,
    );
    view.move_next();
    assert_eq!(
        view.selected_item().unwrap().entry.path,
        std::path::Path::new("b.rs")
    );
    view.move_next();
    assert_eq!(
        view.selected_item().unwrap().entry.path,
        std::path::Path::new("a.rs")
    );
    view.move_previous();
    assert_eq!(
        view.selected_item().unwrap().entry.path,
        std::path::Path::new("b.rs")
    );
}

#[test]
fn staged_entries_cannot_be_discarded_directly() {
    let mut view = GitStatusView::new(
        &status(vec![entry(
            "a.rs",
            StatusCode::Modified,
            StatusCode::Unmodified,
            EntryKind::Ordinary,
        )]),
        None,
    );
    assert!(view.request_discard().is_none());
}
