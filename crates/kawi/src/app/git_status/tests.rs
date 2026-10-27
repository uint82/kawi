use super::*;
use kawi_git::{BranchStatus, OperationState, Status, Upstream};

const REPO: &str = "/repo";

fn status(branch: BranchStatus) -> Status {
    Status {
        root: PathBuf::from(REPO),
        git_dir: PathBuf::from("/repo/.git"),
        branch,
        operation: OperationState::None,
        entries: Vec::new(),
    }
}

fn state(status: Status) -> GitStatusState {
    GitStatusState {
        status: Some(status),
        root: Some(PathBuf::from(REPO)),
        ..GitStatusState::default()
    }
}

#[test]
fn display_uses_branch_and_divergence() {
    let state = state(status(BranchStatus::Named {
        name: "main".to_string(),
        oid: Some("abcdef".to_string()),
        upstream: Some(Upstream {
            name: "origin/main".to_string(),
            ahead: 3,
            behind: 2,
        }),
    }));
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
    let state = state(status);
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

#[test]
fn unborn_branch_has_no_divergence() {
    let state = state(status(BranchStatus::Named {
        name: "main".to_string(),
        oid: None,
        upstream: None,
    }));
    let display = state.display().unwrap();
    assert_eq!(display.branch, "main");
    assert_eq!(display.ahead, 0);
    assert_eq!(display.behind, 0);
}

#[test]
fn missing_status_hides_the_git_statusline() {
    assert!(GitStatusState::default().display().is_none());
}

#[test]
fn status_from_another_root_is_hidden() {
    let mut state = state(status(BranchStatus::Named {
        name: "main".to_string(),
        oid: Some("abcdef".to_string()),
        upstream: None,
    }));
    state.root = Some(PathBuf::from("/other"));
    assert!(state.display().is_none());
}

#[test]
fn clear_removes_the_statusline() {
    let mut state = state(status(BranchStatus::Named {
        name: "main".to_string(),
        oid: Some("abcdef".to_string()),
        upstream: None,
    }));
    state.clear();
    assert!(state.display().is_none());
}

#[test]
fn git_root_for_requires_a_repository() {
    let dir = crate::test_support::temp_dir("git_root_for");
    let file = dir.join("a.txt");
    std::fs::write(&file, "").expect("write");
    assert_eq!(git_root_for(&file), None);
    std::fs::create_dir(dir.join(".git")).expect("create git dir");
    assert_eq!(
        git_root_for(&file),
        Some(dir.canonicalize().expect("canonical"))
    );
}
