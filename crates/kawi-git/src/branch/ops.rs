use anyhow::Result;
use std::path::Path;

use crate::command::{run, run_read, stderr};
use crate::version::{require_version, GitVersion};

use super::{Branch, BranchKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchMode {
    Normal,
    Stash,
    Merge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwitchOutcome {
    Switched,
    Refused(String),
    Failed(String),
}

pub fn switch(root: &Path, branch: &Branch, mode: SwitchMode) -> Result<SwitchOutcome> {
    if branch.kind == BranchKind::Detached {
        return switch_detached(root, &branch.full_ref);
    }
    if branch
        .worktree
        .as_ref()
        .is_some_and(|path| !same_worktree(root, path))
    {
        return Ok(SwitchOutcome::Refused(format!(
            "Branch {} is checked out in {}",
            branch.name,
            branch.worktree.as_ref().unwrap().display()
        )));
    }
    if mode == SwitchMode::Stash {
        return stash_and_switch(root, branch);
    }
    let version = require_version()?;
    let args = switch_args(version, branch, mode);
    run_switch(root, &args)
}

pub fn switch_detached(root: &Path, target: &str) -> Result<SwitchOutcome> {
    if target.is_empty() || target.starts_with('-') {
        return Ok(SwitchOutcome::Refused(format!(
            "Invalid revision: {target}"
        )));
    }
    let version = require_version()?;
    let args = detached_args(version, target);
    run_switch(root, &args)
}

pub fn switch_ref(root: &Path, target: &str) -> Result<SwitchOutcome> {
    if target.is_empty() || target.starts_with('-') {
        return Ok(SwitchOutcome::Refused(format!(
            "Invalid revision: {target}"
        )));
    }
    if let Some(branch) = find_branch(root, target)? {
        return switch(root, &branch, SwitchMode::Normal);
    }
    switch_detached(root, target)
}

pub(super) fn switch_args(version: GitVersion, branch: &Branch, mode: SwitchMode) -> Vec<String> {
    if version.supports_switch() {
        switch_args_switch(branch, mode)
    } else {
        switch_args_checkout(branch, mode)
    }
}

fn switch_args_switch(branch: &Branch, mode: SwitchMode) -> Vec<String> {
    let mut args = vec!["switch".to_string()];
    if mode == SwitchMode::Merge {
        args.push("--merge".to_string());
    }
    match branch.kind {
        BranchKind::Local => {
            args.push("--".to_string());
            args.push(branch.name.clone());
        }
        BranchKind::Remote => {
            args.push("--track".to_string());
            args.push("--".to_string());
            args.push(branch.name.clone());
        }
        BranchKind::Detached => {
            args.push("--detach".to_string());
            args.push("--".to_string());
            args.push(branch.full_ref.clone());
        }
    }
    args
}

fn switch_args_checkout(branch: &Branch, mode: SwitchMode) -> Vec<String> {
    let mut args = vec!["checkout".to_string()];
    if mode == SwitchMode::Merge {
        args.push("-m".to_string());
    }
    match branch.kind {
        BranchKind::Local => args.push(branch.name.clone()),
        BranchKind::Remote => {
            args.push("--track".to_string());
            args.push(branch.name.clone());
        }
        BranchKind::Detached => {
            args.push("--detach".to_string());
            args.push(branch.full_ref.clone());
        }
    }
    args
}

fn detached_args(version: GitVersion, target: &str) -> Vec<String> {
    if version.supports_switch() {
        vec![
            "switch".to_string(),
            "--detach".to_string(),
            "--".to_string(),
            target.to_string(),
        ]
    } else {
        vec![
            "checkout".to_string(),
            "--detach".to_string(),
            target.to_string(),
        ]
    }
}

fn run_switch(root: &Path, args: &[String]) -> Result<SwitchOutcome> {
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = run(root, &refs)?;
    if output.status.success() {
        return Ok(SwitchOutcome::Switched);
    }
    let message = stderr(&output);
    if is_refusal(&message) {
        Ok(SwitchOutcome::Refused(message))
    } else {
        Ok(SwitchOutcome::Failed(message))
    }
}

fn stash_and_switch(root: &Path, branch: &Branch) -> Result<SwitchOutcome> {
    let message = format!("kawi: switch to {}", branch.name);
    let output = run(
        root,
        &["stash", "push", "--include-untracked", "-m", &message],
    )?;
    if !output.status.success() {
        return Ok(SwitchOutcome::Failed(stderr(&output)));
    }
    let version = require_version()?;
    let args = switch_args(version, branch, SwitchMode::Normal);
    match run_switch(root, &args)? {
        SwitchOutcome::Switched => Ok(SwitchOutcome::Switched),
        SwitchOutcome::Refused(message) => Ok(SwitchOutcome::Refused(format!(
            "{message}\nChanges were stashed"
        ))),
        SwitchOutcome::Failed(message) => Ok(SwitchOutcome::Failed(message)),
    }
}

fn find_branch(root: &Path, target: &str) -> Result<Option<Branch>> {
    let local = format!("refs/heads/{target}");
    if ref_exists(root, &local)? {
        return Ok(Some(branch_from_ref(target, local, BranchKind::Local)));
    }
    let remote = format!("refs/remotes/{target}");
    if ref_exists(root, &remote)? {
        return Ok(Some(branch_from_ref(target, remote, BranchKind::Remote)));
    }
    Ok(None)
}

fn ref_exists(root: &Path, reference: &str) -> Result<bool> {
    let output = run_read(root, &["show-ref", "--verify", "--quiet", reference])?;
    Ok(output.status.success())
}

fn branch_from_ref(name: &str, full_ref: String, kind: BranchKind) -> Branch {
    Branch {
        name: name.to_string(),
        full_ref,
        oid: String::new(),
        upstream: None,
        is_current: false,
        worktree: None,
        committer_unix: 0,
        kind,
    }
}

fn same_worktree(root: &Path, path: &Path) -> bool {
    std::fs::canonicalize(root).ok() == std::fs::canonicalize(path).ok()
}

fn is_refusal(message: &str) -> bool {
    message.contains("would be overwritten")
        || message.contains("Please commit your changes or stash them")
        || message.contains("Your local changes")
}
