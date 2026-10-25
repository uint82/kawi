use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub(crate) fn run(root: &Path, args: &[&str]) -> Result<Output> {
    git_command(root)
        .args(args)
        .output()
        .context("failed to run git")
}

pub(crate) fn run_read(root: &Path, args: &[&str]) -> Result<Output> {
    read_command(root)
        .args(args)
        .output()
        .context("failed to run git")
}

pub(crate) fn run_global_read(args: &[&str]) -> Result<Output> {
    let mut command = Command::new("git");
    command
        .arg("--no-optional-locks")
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("LC_ALL", "C");
    command.output().context("failed to run git")
}

pub(crate) fn run_status(root: &Path, args: &[&str]) -> Result<Output> {
    read_command(root)
        .args(args)
        .output()
        .context("failed to run git status")
}

fn read_command(root: &Path) -> Command {
    let mut command = git_command(root);
    command
        .arg("--no-optional-locks")
        .env("GIT_OPTIONAL_LOCKS", "0");
    command
}

fn git_command(root: &Path) -> Command {
    let mut command = Command::new("git");
    command.arg("-C").arg(root).env("LC_ALL", "C");
    command
}

pub(crate) fn path_from_output(bytes: &[u8]) -> PathBuf {
    let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    let bytes = bytes.strip_suffix(b"\r").unwrap_or(bytes);
    path_from_bytes(bytes)
}

pub(crate) fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).trim().to_string()
}

pub fn resolve_root(path: &Path) -> Result<Option<PathBuf>> {
    let start = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or_else(|| Path::new("."))
    };
    let output = run_read(start, &["rev-parse", "--show-toplevel"])?;
    if !output.status.success() {
        return Ok(None);
    }
    let root = path_from_output(&output.stdout);
    Ok(Some(fs::canonicalize(&root).unwrap_or(root)))
}

pub(crate) fn resolve_git_dir(root: &Path) -> Result<Option<PathBuf>> {
    let output = run_read(root, &["rev-parse", "--absolute-git-dir"])?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_commands_do_not_take_optional_locks() {
        let command = read_command(Path::new("/tmp"));
        assert!(command
            .get_args()
            .any(|argument| argument == "--no-optional-locks"));
        assert!(command.get_envs().any(|(name, value)| {
            name == "GIT_OPTIONAL_LOCKS" && value == Some(std::ffi::OsStr::new("0"))
        }));
    }

    #[test]
    fn parsed_commands_use_the_c_locale() {
        let command = read_command(Path::new("/tmp"));
        assert!(command
            .get_envs()
            .any(|(name, value)| name == "LC_ALL" && value == Some(std::ffi::OsStr::new("C"))));
    }
}
