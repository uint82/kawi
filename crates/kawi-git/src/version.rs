use std::sync::OnceLock;

use anyhow::{Context, Result};

use crate::command::run_global_read;

const SWITCH_VERSION: GitVersion = GitVersion::new(2, 23, 0);
const WORKTREE_PATH_VERSION: GitVersion = GitVersion::new(2, 23, 0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GitVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl GitVersion {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        let version = text.trim().strip_prefix("git version ")?;
        let mut parts = version.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts
            .next()
            .and_then(|part| {
                part.split(|character: char| !character.is_ascii_digit())
                    .next()
            })
            .and_then(|part| part.parse().ok())
            .unwrap_or_default();
        Some(Self::new(major, minor, patch))
    }

    pub fn supports_switch(self) -> bool {
        self >= SWITCH_VERSION
    }

    pub fn supports_worktree_path(self) -> bool {
        self >= WORKTREE_PATH_VERSION
    }
}

pub fn git_version() -> Option<GitVersion> {
    static VERSION: OnceLock<Option<GitVersion>> = OnceLock::new();
    *VERSION.get_or_init(detect_version)
}

fn detect_version() -> Option<GitVersion> {
    let output = run_global_read(&["--version"]).ok()?;
    if !output.status.success() {
        return None;
    }
    GitVersion::parse(&String::from_utf8_lossy(&output.stdout))
}

pub(crate) fn require_version() -> Result<GitVersion> {
    git_version().context("git is not available")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_release_versions() {
        assert_eq!(
            GitVersion::parse("git version 2.43.0\n"),
            Some(GitVersion::new(2, 43, 0))
        );
        assert_eq!(
            GitVersion::parse("git version 2.23.4.windows.1"),
            Some(GitVersion::new(2, 23, 4))
        );
        assert_eq!(
            GitVersion::parse("git version 2.7.4"),
            Some(GitVersion::new(2, 7, 4))
        );
    }

    #[test]
    fn rejects_unknown_versions() {
        assert_eq!(GitVersion::parse("hg version 6.0"), None);
        assert_eq!(GitVersion::parse("git version"), None);
    }

    #[test]
    fn gates_version_dependent_features() {
        assert!(GitVersion::new(2, 23, 0).supports_switch());
        assert!(GitVersion::new(2, 23, 0).supports_worktree_path());
        assert!(!GitVersion::new(2, 22, 9).supports_switch());
        assert!(!GitVersion::new(2, 22, 9).supports_worktree_path());
    }
}
