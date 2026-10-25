use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::command::path_from_bytes;

use super::{Branch, BranchKind};

const LOCAL_PREFIX: &str = "refs/heads/";
const REMOTE_PREFIX: &str = "refs/remotes/";

pub(super) fn parse_ref_list(bytes: &[u8], has_worktree: bool) -> Result<Vec<Branch>> {
    let mut branches = Vec::new();
    for line in bytes.split(|byte| *byte == b'\n') {
        if line.is_empty() {
            continue;
        }
        branches.push(parse_ref(line, has_worktree)?);
    }
    Ok(branches)
}

fn parse_ref(line: &[u8], has_worktree: bool) -> Result<Branch> {
    let fields: Vec<&[u8]> = line.split(|byte| *byte == 0).collect();
    let expected = if has_worktree { 7 } else { 6 };
    if fields.len() != expected {
        anyhow::bail!("invalid branch reference record");
    }
    let full_ref = text(fields[1])?;
    let kind = branch_kind(&full_ref).context("unsupported branch reference")?;
    Ok(Branch {
        name: text(fields[0])?,
        full_ref,
        oid: text(fields[2])?,
        upstream: non_empty(text(fields[3])?),
        is_current: fields[4] == b"*",
        committer_unix: parse_date(fields[5])?,
        worktree: has_worktree.then(|| non_empty_path(fields[6])).flatten(),
        kind,
    })
}

fn branch_kind(full_ref: &str) -> Option<BranchKind> {
    if full_ref.starts_with(LOCAL_PREFIX) {
        Some(BranchKind::Local)
    } else if full_ref.starts_with(REMOTE_PREFIX) {
        Some(BranchKind::Remote)
    } else {
        None
    }
}

fn text(bytes: &[u8]) -> Result<String> {
    std::str::from_utf8(bytes)
        .map(str::to_string)
        .context("invalid branch reference")
}

fn non_empty(text: String) -> Option<String> {
    (!text.is_empty()).then_some(text)
}

fn non_empty_path(bytes: &[u8]) -> Option<PathBuf> {
    (!bytes.is_empty()).then(|| path_from_bytes(bytes))
}

fn parse_date(bytes: &[u8]) -> Result<i64> {
    std::str::from_utf8(bytes)
        .context("invalid branch date")?
        .parse()
        .context("invalid branch date")
}

pub(super) fn parse_worktree_list(bytes: &[u8]) -> HashMap<String, PathBuf> {
    let mut paths = HashMap::new();
    let mut current = None;
    for line in bytes.split(|byte| *byte == b'\n') {
        if let Some(path) = line.strip_prefix(b"worktree ") {
            current = Some(path_from_bytes(path));
        } else if let Some(branch) = line.strip_prefix(b"branch ") {
            if let (Some(path), Ok(branch)) = (current.clone(), std::str::from_utf8(branch)) {
                paths.insert(branch.to_string(), path);
            }
        }
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_local_remote_and_current_refs() {
        let input =
            b"feat/one\0refs/heads/feat/one\0abc1234\0origin/feat/one\0*\x001790000000\0/tmp/wt\n\
origin/main\0refs/remotes/origin/main\0def5678\0\0 \x001790000001\0\n";
        let branches = parse_ref_list(input, true).unwrap();
        assert_eq!(branches.len(), 2);
        assert_eq!(branches[0].name, "feat/one");
        assert_eq!(branches[0].kind, BranchKind::Local);
        assert_eq!(branches[0].upstream.as_deref(), Some("origin/feat/one"));
        assert!(branches[0].is_current);
        assert_eq!(
            branches[0].worktree.as_deref(),
            Some(std::path::Path::new("/tmp/wt"))
        );
        assert_eq!(branches[1].kind, BranchKind::Remote);
        assert!(!branches[1].is_current);
    }

    #[test]
    fn parses_refs_without_worktree_paths() {
        let input = b"main\0refs/heads/main\0abc1234\0\0*\x001790000000\n";
        let branches = parse_ref_list(input, false).unwrap();
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].worktree, None);
    }

    #[test]
    fn rejects_malformed_records() {
        assert!(parse_ref_list(b"main\0refs/heads/main\n", true).is_err());
    }

    #[test]
    fn parses_worktree_paths_by_branch() {
        let input = b"worktree /tmp/one\nHEAD abc\nbranch refs/heads/main\n\nworktree /tmp/two\nHEAD def\ndetached\n";
        let paths = parse_worktree_list(input);
        assert_eq!(
            paths.get("refs/heads/main").map(PathBuf::as_path),
            Some(std::path::Path::new("/tmp/one"))
        );
        assert_eq!(paths.len(), 1);
    }
}
