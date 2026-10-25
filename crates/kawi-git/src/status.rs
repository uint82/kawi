use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::command::{resolve_git_dir, resolve_root, run_status, stderr};

mod parser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub root: PathBuf,
    pub git_dir: PathBuf,
    pub branch: BranchStatus,
    pub operation: OperationState,
    pub entries: Vec<StatusEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BranchStatus {
    Named {
        name: String,
        oid: Option<String>,
        upstream: Option<Upstream>,
    },
    Detached {
        oid: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Upstream {
    pub name: String,
    pub ahead: usize,
    pub behind: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationState {
    None,
    Merge,
    Rebase,
    CherryPick,
    Revert,
    Bisect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusCode {
    Unmodified,
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Unmerged,
    Untracked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Ordinary,
    Renamed,
    Unmerged,
    Untracked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusEntry {
    pub path: PathBuf,
    pub original_path: Option<PathBuf>,
    pub index: StatusCode,
    pub worktree: StatusCode,
    pub kind: EntryKind,
}

impl Status {
    pub fn load(path: &Path) -> Result<Option<Self>> {
        let Some(root) = resolve_root(path)? else {
            return Ok(None);
        };
        let Some(git_dir) = resolve_git_dir(&root)? else {
            return Ok(None);
        };
        let output = run_status(&root, &["status", "--porcelain=v2", "--branch", "-z"])?;
        if !output.status.success() {
            anyhow::bail!("git status failed: {}", stderr(&output));
        }
        let mut status = parse_status(&output.stdout)?;
        status.root = root;
        status.git_dir = git_dir.clone();
        status.operation = detect_operation(&git_dir);
        Ok(Some(status))
    }

    pub fn has_staged(&self) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.has_staged() && entry.kind != EntryKind::Unmerged)
    }

    pub fn has_conflicts(&self) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.kind == EntryKind::Unmerged)
    }

    pub fn branch_label(&self) -> &str {
        match &self.branch {
            BranchStatus::Named { name, .. } => name,
            BranchStatus::Detached { oid } => oid,
        }
    }

    pub fn is_unborn(&self) -> bool {
        matches!(&self.branch, BranchStatus::Named { oid: None, .. })
    }

    pub fn upstream(&self) -> Option<&Upstream> {
        match &self.branch {
            BranchStatus::Named { upstream, .. } => upstream.as_ref(),
            BranchStatus::Detached { .. } => None,
        }
    }

    pub fn is_detached(&self) -> bool {
        matches!(self.branch, BranchStatus::Detached { .. })
    }
}

impl BranchStatus {
    pub fn short_oid(&self) -> Option<&str> {
        match self {
            Self::Named { oid, .. } => oid.as_deref(),
            Self::Detached { oid } => Some(oid),
        }
    }
}

impl OperationState {
    pub fn label(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Merge => Some("MERGING"),
            Self::Rebase => Some("REBASING"),
            Self::CherryPick => Some("CHERRY-PICKING"),
            Self::Revert => Some("REVERTING"),
            Self::Bisect => Some("BISECTING"),
        }
    }
}

impl StatusCode {
    fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            b'.' => Some(Self::Unmodified),
            b'M' => Some(Self::Modified),
            b'A' => Some(Self::Added),
            b'D' => Some(Self::Deleted),
            b'R' => Some(Self::Renamed),
            b'C' => Some(Self::Copied),
            b'T' => Some(Self::TypeChanged),
            b'U' => Some(Self::Unmerged),
            b'?' => Some(Self::Untracked),
            _ => None,
        }
    }

    pub fn label(self) -> char {
        match self {
            Self::Unmodified => '.',
            Self::Modified => 'M',
            Self::Added => 'A',
            Self::Deleted => 'D',
            Self::Renamed => 'R',
            Self::Copied => 'C',
            Self::TypeChanged => 'T',
            Self::Unmerged => 'U',
            Self::Untracked => '?',
        }
    }
}

impl StatusEntry {
    pub fn has_staged(&self) -> bool {
        self.kind != EntryKind::Untracked && self.index != StatusCode::Unmodified
    }

    pub fn has_unstaged(&self) -> bool {
        self.kind == EntryKind::Untracked || self.worktree != StatusCode::Unmodified
    }

    pub fn is_conflict(&self) -> bool {
        self.kind == EntryKind::Unmerged
    }

    pub fn code(&self) -> String {
        format!("{}{}", self.index.label(), self.worktree.label())
    }
}

use parser::detect_operation;
pub use parser::parse_status;

#[cfg(test)]
mod tests;
