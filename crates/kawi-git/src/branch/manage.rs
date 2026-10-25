use anyhow::Result;
use std::path::Path;

use crate::command::{run, run_read, stderr};

use super::{Branch, BranchKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateOutcome {
    Created,
    Invalid(String),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteOutcome {
    Deleted,
    NeedsForce { tip: String },
    Refused(String),
    Failed(String),
}

pub fn create(root: &Path, name: &str, base: Option<&str>, unborn: bool) -> Result<CreateOutcome> {
    if let Err(message) = validate_branch_name(root, name) {
        return Ok(CreateOutcome::Invalid(message));
    }
    let result = if unborn {
        create_unborn(root, name)
    } else {
        create_from(root, name, base)
    };
    Ok(match result {
        Ok(()) => CreateOutcome::Created,
        Err(message) => CreateOutcome::Failed(message),
    })
}

pub fn delete(root: &Path, branch: &Branch, force: bool) -> Result<DeleteOutcome> {
    if branch.kind != BranchKind::Local {
        return Ok(DeleteOutcome::Refused(
            "Only local branches can be deleted".to_string(),
        ));
    }
    if branch.is_current {
        return Ok(DeleteOutcome::Refused(
            "Cannot delete the current branch".to_string(),
        ));
    }
    if let Some(path) = branch.worktree.as_ref() {
        return Ok(DeleteOutcome::Refused(format!(
            "Branch {} is checked out in {}",
            branch.name,
            path.display()
        )));
    }
    let flag = if force { "-D" } else { "-d" };
    let output = run(root, &["branch", flag, "--", &branch.name])?;
    if output.status.success() {
        return Ok(DeleteOutcome::Deleted);
    }
    let message = stderr(&output);
    if !force && message.contains("not fully merged") {
        let tip = branch_tip(root, &branch.full_ref)?;
        return Ok(DeleteOutcome::NeedsForce { tip });
    }
    Ok(DeleteOutcome::Failed(message))
}

fn validate_branch_name(root: &Path, name: &str) -> Result<(), String> {
    if name.is_empty() || name.starts_with('-') {
        return Err(format!("Invalid branch name: {name}"));
    }
    let output = run_read(root, &["check-ref-format", "--branch", name])
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(stderr(&output))
    }
}

fn create_unborn(root: &Path, name: &str) -> Result<(), String> {
    let reference = format!("refs/heads/{name}");
    let output =
        run(root, &["symbolic-ref", "HEAD", &reference]).map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(stderr(&output))
    }
}

fn create_from(root: &Path, name: &str, base: Option<&str>) -> Result<(), String> {
    let mut args = vec!["branch", "--", name];
    if let Some(base) = base {
        args.push(base);
    }
    let output = run(root, &args).map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(stderr(&output))
    }
}

fn branch_tip(root: &Path, reference: &str) -> Result<String> {
    let output = run_read(root, &["rev-parse", "--short", reference])?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Ok(String::new())
    }
}
