use super::*;
use kawi_git::{BranchStatus, OperationState, Status, Upstream};

fn status(branch: BranchStatus) -> Status {
    Status {
        root: Default::default(),
        git_dir: Default::default(),
        branch,
        operation: OperationState::None,
        entries: Vec::new(),
    }
}

#[test]
fn display_uses_branch_and_divergence() {
    let state = GitStatusState {
        status: Some(status(BranchStatus::Named {
            name: "main".to_string(),
            oid: Some("abcdef".to_string()),
            upstream: Some(Upstream {
                name: "origin/main".to_string(),
                ahead: 3,
                behind: 2,
            }),
        })),
        ..GitStatusState::default()
    };
    let display = state.display().unwrap();
    assert_eq!(display.branch, "main");
    assert_eq!(display.ahead, 3);
    assert_eq!(display.behind, 2);
    assert_eq!(display.operation, None);
}

#[test]
fn display_shortens_detached_heads_and_shows_operations() {
    let mut status = status(BranchStatus::Detached {
        oid: "0123456789abcdef".to_string(),
    });
    status.operation = OperationState::Rebase;
    let state = GitStatusState {
        status: Some(status),
        ..GitStatusState::default()
    };
    let display = state.display().unwrap();
    assert_eq!(display.branch, "0123456");
    assert_eq!(display.operation, Some("REBASING"));
}

#[test]
fn watcher_relevance_is_limited_to_git_state() {
    let git_dir = Path::new("/repo/.git");
    assert!(is_relevant(&git_dir.join("HEAD"), git_dir));
    assert!(is_relevant(&git_dir.join("index"), git_dir));
    assert!(is_relevant(git_dir, git_dir));
    assert!(!is_relevant(&git_dir.join("objects"), git_dir));
}
