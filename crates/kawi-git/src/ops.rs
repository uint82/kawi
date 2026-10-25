use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::command::{resolve_git_dir, stderr};

pub fn git_dir(root: &Path) -> Result<Option<PathBuf>> {
    resolve_git_dir(root)
}

pub fn stage_path(root: &Path, path: &Path) -> Result<()> {
    checked(root, &["add", "-A", "--"], path)
}

pub fn unstage_path(root: &Path, path: &Path, unborn: bool) -> Result<()> {
    if unborn {
        checked(root, &["rm", "--cached", "-q", "--"], path)
    } else {
        checked(root, &["reset", "-q", "HEAD", "--"], path)
    }
}

pub fn restore_path(root: &Path, path: &Path) -> Result<()> {
    checked(root, &["restore", "--worktree", "--"], path)
}

pub fn clean_path(root: &Path, path: &Path) -> Result<()> {
    checked(root, &["clean", "-fd", "--"], path)
}

pub fn diff_path(root: &Path, path: &Path, staged: bool) -> Result<Vec<String>> {
    let output = if staged {
        run_read_with_path(root, &["diff", "--cached", "--no-color", "--"], path)?
    } else {
        run_read_with_path(root, &["diff", "--no-color", "--"], path)?
    };
    if !output.status.success() {
        bail!("git diff failed: {}", stderr(&output));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_string)
        .collect())
}

fn checked(root: &Path, args: &[&str], path: &Path) -> Result<()> {
    let output = run_with_path(root, args, path)?;
    if !output.status.success() {
        bail!("git {} failed: {}", args.join(" "), stderr(&output));
    }
    Ok(())
}

fn run_with_path(root: &Path, args: &[&str], path: &Path) -> Result<Output> {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .arg(path)
        .env("LC_ALL", "C")
        .output()
        .context("failed to run git")
}

fn run_read_with_path(root: &Path, args: &[&str], path: &Path) -> Result<Output> {
    Command::new("git")
        .arg("--no-optional-locks")
        .arg("-C")
        .arg(root)
        .args(args)
        .arg(path)
        .env("LC_ALL", "C")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .context("failed to run git diff")
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
            "kawi_ops_{}_{}_{}",
            std::process::id(),
            name,
            suffix
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-q"]);
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

    #[test]
    fn stages_unstages_and_restores_a_path() {
        let dir = repo("stage");
        let path = dir.join("a.txt");
        fs::write(&path, "one\n").unwrap();
        git(&dir, &["add", "a.txt"]);
        git(&dir, &["commit", "-qm", "one"]);
        fs::write(&path, "two\n").unwrap();
        stage_path(&dir, Path::new("a.txt")).unwrap();
        assert_eq!(git(&dir, &["diff", "--cached", "--name-only"]), "a.txt");
        unstage_path(&dir, Path::new("a.txt"), false).unwrap();
        assert_eq!(git(&dir, &["diff", "--cached", "--name-only"]), "");
        restore_path(&dir, Path::new("a.txt")).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "one\n");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn unstages_on_an_unborn_branch() {
        let dir = repo("unborn");
        fs::write(dir.join("a.txt"), "one\n").unwrap();
        stage_path(&dir, Path::new("a.txt")).unwrap();
        unstage_path(&dir, Path::new("a.txt"), true).unwrap();
        assert_eq!(git(&dir, &["diff", "--cached", "--name-only"]), "");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cleans_untracked_paths_and_reports_diffs() {
        let dir = repo("clean");
        fs::write(dir.join("tracked.txt"), "one\n").unwrap();
        git(&dir, &["add", "tracked.txt"]);
        git(&dir, &["commit", "-qm", "one"]);
        fs::write(dir.join("tracked.txt"), "two\n").unwrap();
        fs::write(dir.join("new.txt"), "new\n").unwrap();
        let diff = diff_path(&dir, Path::new("tracked.txt"), false).unwrap();
        assert!(diff.iter().any(|line| line.contains("+two")));
        clean_path(&dir, Path::new("new.txt")).unwrap();
        assert!(!dir.join("new.txt").exists());
        let _ = fs::remove_dir_all(dir);
    }
}
