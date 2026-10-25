use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::command::{path_from_bytes, run_read, stderr};

const NAME_STATUS_ARGS: &[&str] = &[
    "diff-tree",
    "-r",
    "--no-commit-id",
    "--name-status",
    "-z",
    "-m",
    "--first-parent",
    "--root",
    "-M",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitFile {
    pub status: char,
    pub path: PathBuf,
    pub old_path: Option<PathBuf>,
}

pub fn commit_files(root: &Path, sha: &str) -> Result<Vec<CommitFile>> {
    let mut args = NAME_STATUS_ARGS.to_vec();
    args.push(sha);
    let output = run_read(root, &args)?;
    if !output.status.success() {
        anyhow::bail!("git diff-tree failed: {}", stderr(&output));
    }
    parse_name_status(&output.stdout)
}

pub fn commit_patch(root: &Path, sha: &str) -> Result<Vec<String>> {
    let args = [
        "diff-tree",
        "-r",
        "-p",
        "--no-commit-id",
        "--root",
        "-m",
        "--first-parent",
        "-M",
        sha,
    ];
    let output = run_read(root, &args)?;
    if !output.status.success() {
        anyhow::bail!("git diff-tree failed: {}", stderr(&output));
    }
    Ok(lines(&output.stdout))
}

pub fn file_diff(root: &Path, sha: &str, file: &CommitFile) -> Result<Vec<String>> {
    if has_parent(root, sha)? {
        return parent_diff(root, sha, file);
    }
    root_diff(root, sha, file)
}

fn parent_diff(root: &Path, sha: &str, file: &CommitFile) -> Result<Vec<String>> {
    let parent = format!("{sha}^1");
    let new_path = path_text(&file.path)?;
    let old_path = file
        .old_path
        .as_deref()
        .map(path_text)
        .transpose()?
        .unwrap_or(new_path);
    let output = run_read(
        root,
        &[
            "diff",
            "--no-color",
            "--no-ext-diff",
            "-M",
            &parent,
            sha,
            "--",
            old_path,
            new_path,
        ],
    )?;
    if !output.status.success() {
        anyhow::bail!("git diff failed: {}", stderr(&output));
    }
    Ok(lines(&output.stdout))
}

fn root_diff(root: &Path, sha: &str, file: &CommitFile) -> Result<Vec<String>> {
    let path = path_text(&file.path)?;
    let old_path = file
        .old_path
        .as_deref()
        .map(path_text)
        .transpose()?
        .unwrap_or(path);
    let args = [
        "diff-tree",
        "-r",
        "-p",
        "--no-commit-id",
        "--root",
        "-M",
        sha,
        "--",
        old_path,
        path,
    ];
    let output = run_read(root, &args)?;
    if !output.status.success() {
        anyhow::bail!("git diff-tree failed: {}", stderr(&output));
    }
    Ok(lines(&output.stdout))
}

fn has_parent(root: &Path, sha: &str) -> Result<bool> {
    let output = run_read(
        root,
        &["rev-parse", "--verify", "--quiet", &format!("{sha}^1")],
    )?;
    Ok(output.status.success())
}

fn path_text(path: &Path) -> Result<&str> {
    path.to_str().context("path is not valid UTF-8")
}

fn lines(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::to_string)
        .collect()
}

fn parse_name_status(bytes: &[u8]) -> Result<Vec<CommitFile>> {
    let mut tokens = bytes
        .split(|byte| *byte == 0)
        .filter(|token| !token.is_empty());
    let mut files = Vec::new();
    while let Some(status) = tokens.next() {
        let status = std::str::from_utf8(status).context("invalid diff status")?;
        let code = status.chars().next().context("empty diff status")?;
        let first = path_from_bytes(tokens.next().context("missing diff path")?);
        let (path, old_path) = if matches!(code, 'R' | 'C') {
            let new = path_from_bytes(tokens.next().context("missing rename target")?);
            (new, Some(first))
        } else {
            (first, None)
        };
        files.push(CommitFile {
            status: code,
            path,
            old_path,
        });
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modified_added_deleted_and_renamed_files() {
        let input = b"M\0src/a.rs\0A\0src/b.rs\0D\0old.rs\0R100\0old/name.rs\0new/name.rs\0";
        let files = parse_name_status(input).unwrap();
        assert_eq!(files.len(), 4);
        assert_eq!(files[0].status, 'M');
        assert_eq!(files[3].status, 'R');
        assert_eq!(files[3].old_path.as_deref(), Some(Path::new("old/name.rs")));
        assert_eq!(files[3].path, Path::new("new/name.rs"));
    }
}
