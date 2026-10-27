use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use super::*;
use crate::app::undo_test_support::app_with;
use crate::app::App;
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

fn state(path: &str, status: Status) -> GitStatusState {
    GitStatusState {
        path: Some(PathBuf::from(path)),
        status: Some(status),
        ..GitStatusState::default()
    }
}

#[test]
fn display_uses_branch_and_divergence() {
    let state = state(
        "/repo/src/main.rs",
        status(BranchStatus::Named {
            name: "main".to_string(),
            oid: Some("abcdef".to_string()),
            upstream: Some(Upstream {
                name: "origin/main".to_string(),
                ahead: 3,
                behind: 2,
            }),
        }),
    );
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
    let state = state("/repo/main.rs", status);
    let display = state.display().unwrap();
    assert_eq!(display.branch, "0123456");
    assert_eq!(display.operation, Some("REBASING"));
}

#[test]
fn unborn_branch_has_no_divergence() {
    let state = state(
        "/repo/main.rs",
        status(BranchStatus::Named {
            name: "main".to_string(),
            oid: None,
            upstream: None,
        }),
    );
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
fn status_outside_the_active_path_is_hidden() {
    let state = state(
        "/other/main.rs",
        status(BranchStatus::Named {
            name: "main".to_string(),
            oid: Some("abcdef".to_string()),
            upstream: None,
        }),
    );
    assert!(state.display().is_none());
}

#[test]
fn clear_removes_the_statusline() {
    let mut state = state(
        "/repo/main.rs",
        status(BranchStatus::Named {
            name: "main".to_string(),
            oid: Some("abcdef".to_string()),
            upstream: None,
        }),
    );
    state.clear();
    assert!(state.display().is_none());
    assert!(state.path.is_none());
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(dir)
        .args(args)
        .status()
        .expect("run git");
    assert!(status.success(), "git {args:?} failed");
}

fn wait_for_status(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.git_status.status().is_none() && Instant::now() < deadline {
        app.sync_git_status();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(app.git_status.status().is_some(), "git status timed out");
}

#[test]
fn refresh_loads_the_active_file_repository() {
    let (dir, mut app) = app_with("git_status_repo", "");
    git(&dir, &["init", "-q"]);
    git(&dir, &["symbolic-ref", "HEAD", "refs/heads/master"]);
    app.refresh_git_status();
    wait_for_status(&mut app);
    assert_eq!(app.git_status.display().expect("display").branch, "master");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn refresh_clears_status_outside_a_repository() {
    let (dir, mut app) = app_with("git_status_switch", "");
    git(&dir, &["init", "-q"]);
    git(&dir, &["symbolic-ref", "HEAD", "refs/heads/master"]);
    app.refresh_git_status();
    wait_for_status(&mut app);
    assert_eq!(app.git_status.display().expect("display").branch, "master");

    let outside = crate::test_support::temp_dir("git_status_outside");
    let path = outside.join("a.txt");
    std::fs::write(&path, "").expect("write");
    app.editor
        .edit_file(Some(path.to_str().expect("path")), false)
        .expect("edit");
    app.refresh_git_status();
    assert!(app.git_status.display().is_none());
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(outside);
}

#[test]
fn switching_files_uses_each_repository_branch() {
    let (repo_a, mut app) = app_with("git_status_main", "");
    git(&repo_a, &["init", "-q"]);
    git(&repo_a, &["symbolic-ref", "HEAD", "refs/heads/main"]);
    let repo_b = crate::test_support::temp_dir("git_status_master");
    git(&repo_b, &["init", "-q"]);
    git(&repo_b, &["symbolic-ref", "HEAD", "refs/heads/master"]);
    let file_b = repo_b.join("b.txt");
    std::fs::write(&file_b, "").expect("write");

    app.refresh_git_status();
    wait_for_status(&mut app);
    assert_eq!(app.git_status.display().expect("main").branch, "main");

    app.editor
        .edit_file(Some(file_b.to_str().expect("path")), false)
        .expect("edit master");
    app.refresh_git_status();
    assert!(app.git_status.display().is_none());
    wait_for_status(&mut app);
    assert_eq!(app.git_status.display().expect("master").branch, "master");

    app.editor
        .edit_file(Some(repo_a.join("a.txt").to_str().expect("path")), false)
        .expect("edit main");
    app.refresh_git_status();
    assert!(app.git_status.display().is_none());
    wait_for_status(&mut app);
    assert_eq!(app.git_status.display().expect("main").branch, "main");

    let _ = std::fs::remove_dir_all(repo_a);
    let _ = std::fs::remove_dir_all(repo_b);
}
