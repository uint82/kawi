use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::command::{run_read, stderr};
use crate::version::require_version;

use super::parser::parse_ref_list;
use super::worktree::worktree_paths;

const FORMAT_FIELDS: &str = "%(refname:short)%00%(refname)%00%(objectname:short)%00%(upstream:short)%00%(HEAD)%00%(committerdate:unix)";
const WORKTREE_FIELD: &str = "%00%(worktreepath)";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BranchListOptions {
    pub pin_current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchList {
    pub branches: Vec<Branch>,
    pub detached: Option<Branch>,
    pub unborn: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    pub name: String,
    pub full_ref: String,
    pub oid: String,
    pub upstream: Option<String>,
    pub is_current: bool,
    pub worktree: Option<PathBuf>,
    pub committer_unix: i64,
    pub kind: BranchKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchKind {
    Local,
    Remote,
    Detached,
}

impl BranchList {
    pub fn is_empty(&self) -> bool {
        self.branches.is_empty() && self.detached.is_none()
    }
}

pub fn is_unborn(root: &Path) -> Result<bool> {
    if has_head(root)? {
        return Ok(false);
    }
    let symbolic = run_read(root, &["symbolic-ref", "-q", "HEAD"])?;
    Ok(symbolic.status.success())
}

pub fn list(root: &Path, options: BranchListOptions) -> Result<BranchList> {
    let version = require_version()?;
    let has_worktree = version.supports_worktree_path();
    let format = branch_format(has_worktree);
    let output = run_read(
        root,
        &[
            "for-each-ref",
            "--sort=-committerdate",
            &format!("--format={format}"),
            "refs/heads",
            "refs/remotes",
        ],
    )?;
    if !output.status.success() {
        anyhow::bail!("git for-each-ref failed: {}", stderr(&output));
    }
    let mut branches = parse_ref_list(&output.stdout, has_worktree)?;
    if !has_worktree {
        apply_worktree_paths(root, &mut branches)?;
    }
    hide_remote_head(&mut branches);
    fold_remote_tracking(&mut branches);
    let detached = detached_entry(root)?;
    let unborn = detached.is_none() && !has_head(root)?;
    let mut branches = order_branches(branches, options.pin_current);
    if unborn {
        branches.clear();
    }
    Ok(BranchList {
        branches,
        detached,
        unborn,
    })
}

fn branch_format(has_worktree: bool) -> String {
    if has_worktree {
        format!("{FORMAT_FIELDS}{WORKTREE_FIELD}")
    } else {
        FORMAT_FIELDS.to_string()
    }
}

fn apply_worktree_paths(root: &Path, branches: &mut [Branch]) -> Result<()> {
    let paths = worktree_paths(root)?;
    for branch in branches {
        branch.worktree = paths.get(&branch.full_ref).cloned();
    }
    Ok(())
}

fn hide_remote_head(branches: &mut Vec<Branch>) {
    branches.retain(|branch| branch.kind != BranchKind::Remote || !branch.name.ends_with("/HEAD"));
}

fn fold_remote_tracking(branches: &mut Vec<Branch>) {
    let tracked: HashSet<String> = branches
        .iter()
        .filter_map(|branch| branch.upstream.clone())
        .collect();
    branches.retain(|branch| branch.kind != BranchKind::Remote || !tracked.contains(&branch.name));
}

fn detached_entry(root: &Path) -> Result<Option<Branch>> {
    let symbolic = run_read(root, &["symbolic-ref", "-q", "HEAD"])?;
    if symbolic.status.success() {
        return Ok(None);
    }
    let output = run_read(root, &["rev-parse", "--short", "HEAD"])?;
    if !output.status.success() {
        return Ok(None);
    }
    let oid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if oid.is_empty() {
        return Ok(None);
    }
    Ok(Some(Branch {
        name: format!("detached at {oid}"),
        full_ref: oid.clone(),
        oid,
        upstream: None,
        is_current: true,
        worktree: None,
        committer_unix: 0,
        kind: BranchKind::Detached,
    }))
}

fn has_head(root: &Path) -> Result<bool> {
    let output = run_read(root, &["rev-parse", "--verify", "--quiet", "HEAD"])?;
    Ok(output.status.success())
}

fn order_branches(mut branches: Vec<Branch>, pin_current: bool) -> Vec<Branch> {
    if pin_current {
        if let Some(index) = branches.iter().position(|branch| branch.is_current) {
            let current = branches.remove(index);
            branches.insert(0, current);
        }
    }
    branches
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    fn repo(name: &str) -> PathBuf {
        let suffix = NEXT.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "kawi_branch_list_{}_{}_{}",
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

    #[test]
    fn lists_local_and_remote_branches_without_remote_head() {
        let dir = repo("list");
        commit(&dir, "a.txt", "one");
        git(&dir, &["branch", "feat/slash"]);
        git(&dir, &["branch", "feat/ü"]);
        git(&dir, &["branch", "feat/HEAD"]);
        let remote = dir.join("remote.git");
        fs::create_dir_all(&remote).unwrap();
        git(&remote, &["init", "-q", "--bare"]);
        git(&dir, &["remote", "add", "origin", remote.to_str().unwrap()]);
        git(&dir, &["push", "-qu", "origin", "main"]);
        git(&dir, &["fetch", "-q", "origin"]);
        let list = list(&dir, BranchListOptions::default()).unwrap();
        assert!(list
            .branches
            .iter()
            .any(|branch| branch.name == "feat/slash"));
        assert!(list.branches.iter().any(|branch| branch.name == "feat/ü"));
        assert!(list
            .branches
            .iter()
            .any(|branch| branch.name == "feat/HEAD"));
        assert!(!list
            .branches
            .iter()
            .any(|branch| branch.kind == BranchKind::Remote && branch.name.ends_with("/HEAD")));
        assert!(!list
            .branches
            .iter()
            .any(|branch| branch.name == "origin/main"));
        assert!(list.detached.is_none());
        assert!(!list.unborn);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn detects_detached_and_unborn_states() {
        let dir = repo("states");
        let unborn = list(&dir, BranchListOptions::default()).unwrap();
        assert!(unborn.unborn);
        assert!(unborn.is_empty());
        commit(&dir, "a.txt", "one");
        git(&dir, &["checkout", "-q", "--detach", "HEAD"]);
        let detached = list(&dir, BranchListOptions::default()).unwrap();
        assert!(detached.detached.is_some());
        assert!(detached.detached.unwrap().name.starts_with("detached at "));
        let _ = fs::remove_dir_all(dir);
    }
}
