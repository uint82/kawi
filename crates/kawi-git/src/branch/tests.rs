use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::version::GitVersion;

use super::ops::switch_args;
use super::*;

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn repo(name: &str) -> PathBuf {
    let suffix = NEXT.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "kawi_branch_ops_{}_{}_{}",
        std::process::id(),
        name,
        suffix
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "Test"]);
    dir
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?} failed");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn commit(dir: &Path, name: &str, text: &str) {
    fs::write(dir.join(name), text).unwrap();
    git(dir, &["add", name]);
    git(dir, &["commit", "-qm", text]);
}

fn branch(dir: &Path, name: &str) -> Branch {
    list(dir, BranchListOptions::default())
        .unwrap()
        .branches
        .into_iter()
        .find(|branch| branch.name == name)
        .unwrap()
}

#[test]
fn switches_local_branches_and_reports_current() {
    let dir = repo("local");
    commit(&dir, "a.txt", "one");
    git(&dir, &["branch", "feat/x"]);
    let target = branch(&dir, "feat/x");
    assert!(matches!(
        switch(&dir, &target, SwitchMode::Normal).unwrap(),
        SwitchOutcome::Switched
    ));
    assert_eq!(git(&dir, &["branch", "--show-current"]), "feat/x");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn switches_remote_only_branches_with_tracking() {
    let dir = repo("remote");
    let remote = dir.join("remote.git");
    fs::create_dir_all(&remote).unwrap();
    git(&remote, &["init", "-q", "--bare"]);
    commit(&dir, "a.txt", "one");
    git(&dir, &["remote", "add", "origin", remote.to_str().unwrap()]);
    git(&dir, &["push", "-qu", "origin", "main"]);
    git(&dir, &["checkout", "-qb", "feat/remote"]);
    commit(&dir, "b.txt", "two");
    git(&dir, &["push", "-qu", "origin", "feat/remote"]);
    git(&dir, &["checkout", "-q", "main"]);
    git(&dir, &["branch", "-D", "feat/remote"]);
    git(&dir, &["fetch", "-q"]);
    let target = branch(&dir, "origin/feat/remote");
    assert!(matches!(
        switch(&dir, &target, SwitchMode::Normal).unwrap(),
        SwitchOutcome::Switched
    ));
    assert_eq!(git(&dir, &["branch", "--show-current"]), "feat/remote");
    assert_eq!(
        git(
            &dir,
            &["rev-parse", "--abbrev-ref", "feat/remote@{upstream}"]
        ),
        "origin/feat/remote"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn refuses_conflicting_switches_and_stashes() {
    let dir = repo("refusal");
    commit(&dir, "a.txt", "one");
    git(&dir, &["branch", "other"]);
    fs::write(dir.join("a.txt"), "two\n").unwrap();
    git(&dir, &["commit", "-qam", "two"]);
    git(&dir, &["checkout", "-q", "other"]);
    fs::write(dir.join("a.txt"), "three\n").unwrap();
    let target = branch(&dir, "main");
    assert!(matches!(
        switch(&dir, &target, SwitchMode::Normal).unwrap(),
        SwitchOutcome::Refused(_)
    ));
    assert!(matches!(
        switch(&dir, &target, SwitchMode::Stash).unwrap(),
        SwitchOutcome::Switched
    ));
    assert_eq!(git(&dir, &["branch", "--show-current"]), "main");
    assert!(git(&dir, &["stash", "list"]).contains("kawi: switch to main"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn creates_from_head_and_unborn_branches() {
    let dir = repo("create");
    commit(&dir, "a.txt", "one");
    assert!(matches!(
        create(&dir, "feat/new", None, false).unwrap(),
        CreateOutcome::Created
    ));
    assert_eq!(git(&dir, &["branch", "--show-current"]), "main");
    assert!(git(&dir, &["branch", "--list", "feat/new"]).contains("feat/new"));
    assert!(matches!(
        create(&dir, "-bad", None, false).unwrap(),
        CreateOutcome::Invalid(_)
    ));
    let unborn = repo("unborn");
    assert!(matches!(
        create(&unborn, "first", None, true).unwrap(),
        CreateOutcome::Created
    ));
    assert_eq!(git(&unborn, &["symbolic-ref", "HEAD"]), "refs/heads/first");
    let _ = fs::remove_dir_all(dir);
    let _ = fs::remove_dir_all(unborn);
}

#[test]
fn deletes_merged_branches_and_requires_force_for_unmerged() {
    let dir = repo("delete");
    commit(&dir, "a.txt", "one");
    git(&dir, &["branch", "merged"]);
    assert!(matches!(
        delete(&dir, &branch(&dir, "merged"), false).unwrap(),
        DeleteOutcome::Deleted
    ));
    git(&dir, &["checkout", "-qb", "unmerged"]);
    commit(&dir, "b.txt", "two");
    git(&dir, &["checkout", "-q", "main"]);
    let target = branch(&dir, "unmerged");
    let outcome = delete(&dir, &target, false).unwrap();
    assert!(matches!(outcome, DeleteOutcome::NeedsForce { .. }));
    assert!(matches!(
        delete(&dir, &target, true).unwrap(),
        DeleteOutcome::Deleted
    ));
    let current = branch(&dir, "main");
    assert!(matches!(
        delete(&dir, &current, false).unwrap(),
        DeleteOutcome::Refused(_)
    ));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn labels_and_refuses_branches_checked_out_in_worktrees() {
    let dir = repo("worktree");
    commit(&dir, "a.txt", "one");
    git(&dir, &["branch", "linked"]);
    let worktree = dir.join("linked-worktree");
    git(
        &dir,
        &[
            "worktree",
            "add",
            "-q",
            worktree.to_str().unwrap(),
            "linked",
        ],
    );
    let target = branch(&dir, "linked");
    assert_eq!(target.worktree.as_deref(), Some(worktree.as_path()));
    assert!(matches!(
        switch(&dir, &target, SwitchMode::Normal).unwrap(),
        SwitchOutcome::Refused(_)
    ));
    assert!(matches!(
        delete(&dir, &target, false).unwrap(),
        DeleteOutcome::Refused(_)
    ));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn selects_switch_and_checkout_arguments_by_version() {
    let local = Branch {
        name: "feat/x".to_string(),
        full_ref: "refs/heads/feat/x".to_string(),
        oid: String::new(),
        upstream: None,
        is_current: false,
        worktree: None,
        committer_unix: 0,
        kind: BranchKind::Local,
    };
    let modern = switch_args(GitVersion::new(2, 23, 0), &local, SwitchMode::Normal);
    assert_eq!(modern, ["switch", "--", "feat/x"]);
    let legacy = switch_args(GitVersion::new(2, 22, 0), &local, SwitchMode::Normal);
    assert_eq!(legacy, ["checkout", "feat/x"]);
    let merge = switch_args(GitVersion::new(2, 23, 0), &local, SwitchMode::Merge);
    assert_eq!(merge, ["switch", "--merge", "--", "feat/x"]);
}
