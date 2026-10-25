use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub(crate) fn run(root: &Path, args: &[&str]) -> Result<Output> {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .context("failed to run git")
}

pub(crate) fn run_status(root: &Path, args: &[&str]) -> Result<Output> {
    Command::new("git")
        .arg("--no-optional-locks")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .context("failed to run git status")
}

pub(crate) fn path_from_output(bytes: &[u8]) -> PathBuf {
    let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    let bytes = bytes.strip_suffix(b"\r").unwrap_or(bytes);
    path_from_bytes(bytes)
}

pub(crate) fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).trim().to_string()
}

pub(crate) fn resolve_root(path: &Path) -> Result<Option<PathBuf>> {
    let start = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or_else(|| Path::new("."))
    };
    let output = run(start, &["rev-parse", "--show-toplevel"])?;
    if !output.status.success() {
        return Ok(None);
    }
    let root = path_from_output(&output.stdout);
    Ok(Some(fs::canonicalize(&root).unwrap_or(root)))
}

pub(crate) fn resolve_git_dir(root: &Path) -> Result<Option<PathBuf>> {
    let output = run(root, &["rev-parse", "--absolute-git-dir"])?;
    if !output.status.success() {
        return Ok(None);
    }
    let dir = path_from_output(&output.stdout);
    Ok(Some(fs::canonicalize(&dir).unwrap_or(dir)))
}

#[cfg(unix)]
pub(crate) fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(OsStr::from_bytes(bytes))
}

#[cfg(not(unix))]
pub(crate) fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
}
