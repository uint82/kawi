use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::command::{path_from_bytes, resolve_git_dir, resolve_root, run_status, stderr};

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

pub fn parse_status(bytes: &[u8]) -> Result<Status> {
    let mut parser = Parser::default();
    let mut tokens = bytes
        .split(|byte| *byte == 0)
        .filter(|token| !token.is_empty());
    while let Some(token) = tokens.next() {
        parser.consume(token, &mut tokens)?;
    }
    parser.finish()
}

#[derive(Default)]
struct Parser {
    branch_name: Option<String>,
    branch_oid: Option<String>,
    upstream: Option<String>,
    ahead: usize,
    behind: usize,
    detached: bool,
    entries: Vec<StatusEntry>,
}

impl Parser {
    fn consume<'a>(
        &mut self,
        token: &'a [u8],
        tokens: &mut impl Iterator<Item = &'a [u8]>,
    ) -> Result<()> {
        match token.first().copied() {
            Some(b'#') => self.header(token),
            Some(b'1') => self.ordinary(token),
            Some(b'2') => {
                let original = tokens.next().context("missing rename source path")?;
                self.renamed(token, original)
            }
            Some(b'u') => self.unmerged(token),
            Some(b'?') => self.untracked(token),
            _ => Ok(()),
        }
    }

    fn header(&mut self, token: &[u8]) -> Result<()> {
        let text = std::str::from_utf8(token).context("invalid status header")?;
        let Some((key, value)) = text[2..].split_once(' ') else {
            return Ok(());
        };
        match key {
            "branch.oid" => {
                self.branch_oid = (value != "(initial)").then(|| value.to_string());
            }
            "branch.head" => {
                if value == "(detached)" {
                    self.detached = true;
                } else {
                    self.branch_name = Some(value.to_string());
                }
            }
            "branch.upstream" => self.upstream = Some(value.to_string()),
            "branch.ab" => self.parse_ahead_behind(value),
            _ => {}
        }
        Ok(())
    }

    fn parse_ahead_behind(&mut self, value: &str) {
        let mut parts = value.split_whitespace();
        self.ahead = parts
            .next()
            .and_then(|part| part.strip_prefix('+'))
            .and_then(|part| part.parse().ok())
            .unwrap_or_default();
        self.behind = parts
            .next()
            .and_then(|part| part.strip_prefix('-'))
            .and_then(|part| part.parse().ok())
            .unwrap_or_default();
    }

    fn ordinary(&mut self, token: &[u8]) -> Result<()> {
        let fields = fields(token, 9)?;
        let (index, worktree) = parse_xy(fields[1])?;
        self.entries.push(StatusEntry {
            path: path_from_bytes(fields[8]),
            original_path: None,
            index,
            worktree,
            kind: EntryKind::Ordinary,
        });
        Ok(())
    }

    fn renamed(&mut self, token: &[u8], original: &[u8]) -> Result<()> {
        let fields = fields(token, 10)?;
        let (index, worktree) = parse_xy(fields[1])?;
        self.entries.push(StatusEntry {
            path: path_from_bytes(fields[9]),
            original_path: Some(path_from_bytes(original)),
            index,
            worktree,
            kind: EntryKind::Renamed,
        });
        Ok(())
    }

    fn unmerged(&mut self, token: &[u8]) -> Result<()> {
        let fields = fields(token, 11)?;
        let (index, worktree) = parse_xy(fields[1])?;
        self.entries.push(StatusEntry {
            path: path_from_bytes(fields[10]),
            original_path: None,
            index,
            worktree,
            kind: EntryKind::Unmerged,
        });
        Ok(())
    }

    fn untracked(&mut self, token: &[u8]) -> Result<()> {
        let path = token
            .strip_prefix(b"? ")
            .context("invalid untracked status entry")?;
        self.entries.push(StatusEntry {
            path: path_from_bytes(path),
            original_path: None,
            index: StatusCode::Untracked,
            worktree: StatusCode::Untracked,
            kind: EntryKind::Untracked,
        });
        Ok(())
    }

    fn finish(self) -> Result<Status> {
        let branch = if self.detached {
            BranchStatus::Detached {
                oid: self.branch_oid.unwrap_or_default(),
            }
        } else {
            BranchStatus::Named {
                name: self.branch_name.unwrap_or_default(),
                oid: self.branch_oid,
                upstream: self.upstream.map(|name| Upstream {
                    name,
                    ahead: self.ahead,
                    behind: self.behind,
                }),
            }
        };
        Ok(Status {
            root: PathBuf::new(),
            git_dir: PathBuf::new(),
            branch,
            operation: OperationState::None,
            entries: self.entries,
        })
    }
}

fn fields(token: &[u8], count: usize) -> Result<Vec<&[u8]>> {
    let fields: Vec<&[u8]> = token.splitn(count, |byte| *byte == b' ').collect();
    if fields.len() != count {
        anyhow::bail!("invalid porcelain v2 status entry");
    }
    Ok(fields)
}

fn parse_xy(bytes: &[u8]) -> Result<(StatusCode, StatusCode)> {
    if bytes.len() != 2 {
        anyhow::bail!("invalid porcelain v2 status code");
    }
    let index = StatusCode::from_byte(bytes[0]).context("invalid index status code")?;
    let worktree = StatusCode::from_byte(bytes[1]).context("invalid worktree status code")?;
    Ok((index, worktree))
}

fn detect_operation(git_dir: &Path) -> OperationState {
    if git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists() {
        return OperationState::Rebase;
    }
    if git_dir.join("MERGE_HEAD").exists() {
        return OperationState::Merge;
    }
    if git_dir.join("CHERRY_PICK_HEAD").exists() {
        return OperationState::CherryPick;
    }
    if git_dir.join("REVERT_HEAD").exists() {
        return OperationState::Revert;
    }
    if git_dir.join("BISECT_LOG").exists() {
        return OperationState::Bisect;
    }
    OperationState::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    fn temp_dir(name: &str) -> PathBuf {
        let suffix = NEXT.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "kawi_status_{}_{}_{}",
            std::process::id(),
            name,
            suffix
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
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

    fn repo(name: &str) -> PathBuf {
        let dir = temp_dir(name);
        git(&dir, &["init", "-q"]);
        git(&dir, &["config", "user.email", "test@example.com"]);
        git(&dir, &["config", "user.name", "Test"]);
        dir
    }

    fn parse(text: &str) -> Status {
        let mut bytes = Vec::new();
        for token in text.split('\0').filter(|token| !token.is_empty()) {
            bytes.extend_from_slice(token.as_bytes());
            bytes.push(0);
        }
        parse_status(&bytes).unwrap()
    }

    #[test]
    fn parses_branch_and_ahead_behind() {
        let status = parse(
            "# branch.oid abcdef\0# branch.head main\0# branch.upstream origin/main\0# branch.ab +2 -3\0",
        );
        assert_eq!(status.branch_label(), "main");
        let upstream = status.upstream().unwrap();
        assert_eq!(upstream.ahead, 2);
        assert_eq!(upstream.behind, 3);
        assert!(!status.is_unborn());
        assert!(!status.is_detached());
    }

    #[test]
    fn parses_detached_and_unborn_branches() {
        let detached = parse("# branch.oid abcdef\0# branch.head (detached)\0");
        assert!(detached.is_detached());
        assert_eq!(detached.branch_label(), "abcdef");
        let unborn = parse("# branch.oid (initial)\0# branch.head main\0");
        assert!(unborn.is_unborn());
        assert_eq!(unborn.branch_label(), "main");
    }

    #[test]
    fn parses_ordinary_renamed_unmerged_and_untracked_entries() {
        let status = parse(
            "1 M. N... 100644 100644 100644 a b src/main.rs\0\
             2 R. N... 100644 100644 100644 a b R100 new name.rs\0old name.rs\0\
             u UU N... 100644 100644 100644 100644 a b c conflict.rs\0\
             ? untracked file.txt\0",
        );
        assert_eq!(status.entries.len(), 4);
        assert_eq!(status.entries[0].path, PathBuf::from("src/main.rs"));
        assert_eq!(status.entries[0].index, StatusCode::Modified);
        assert_eq!(
            status.entries[1].original_path.as_deref(),
            Some(Path::new("old name.rs"))
        );
        assert_eq!(status.entries[1].path, PathBuf::from("new name.rs"));
        assert!(status.entries[2].is_conflict());
        assert_eq!(status.entries[3].kind, EntryKind::Untracked);
        assert_eq!(status.entries[3].path, PathBuf::from("untracked file.txt"));
    }

    #[test]
    fn tracks_both_staged_and_unstaged_changes() {
        let status = parse("1 MM N... 100644 100644 100644 a b a.rs\0");
        let entry = &status.entries[0];
        assert!(entry.has_staged());
        assert!(entry.has_unstaged());
        assert_eq!(entry.code(), "MM");
        assert!(status.has_staged());
        assert!(!status.has_conflicts());
    }

    #[test]
    fn loads_a_real_repository() {
        let dir = repo("real");
        fs::write(dir.join("tracked.txt"), "one\n").unwrap();
        git(&dir, &["add", "tracked.txt"]);
        git(&dir, &["commit", "-qm", "one"]);
        fs::write(dir.join("tracked.txt"), "two\n").unwrap();
        fs::write(dir.join("staged file.txt"), "new\n").unwrap();
        git(&dir, &["add", "staged file.txt"]);
        fs::write(dir.join("untracked file.txt"), "new\n").unwrap();
        let status = Status::load(&dir).unwrap().unwrap();
        assert_eq!(
            status.branch_label(),
            git(&dir, &["branch", "--show-current"])
        );
        assert!(status.has_staged());
        assert!(status.entries.iter().any(|entry| {
            entry.path == Path::new("untracked file.txt") && entry.kind == EntryKind::Untracked
        }));
        assert!(status.entries.iter().any(|entry| {
            entry.path == Path::new("staged file.txt") && entry.index == StatusCode::Added
        }));
        assert!(status.entries.iter().any(|entry| {
            entry.path == Path::new("tracked.txt") && entry.worktree == StatusCode::Modified
        }));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn loads_unborn_and_empty_repository() {
        let dir = repo("unborn");
        let status = Status::load(&dir).unwrap().unwrap();
        assert!(status.is_unborn());
        assert!(!status.is_detached());
        assert!(status.entries.is_empty());
        assert_eq!(
            status.branch_label(),
            git(&dir, &["branch", "--show-current"])
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_and_bare_repositories_are_disabled() {
        let missing = temp_dir("missing");
        assert!(Status::load(&missing).unwrap().is_none());
        let bare = temp_dir("bare");
        git(&bare, &["init", "--bare", "-q"]);
        assert!(Status::load(&bare).unwrap().is_none());
        let _ = fs::remove_dir_all(missing);
        let _ = fs::remove_dir_all(bare);
    }

    #[test]
    fn detects_rebase_and_merge_state() {
        let dir = repo("operation");
        let status = Status::load(&dir).unwrap().unwrap();
        fs::create_dir_all(status.git_dir.join("rebase-merge")).unwrap();
        assert_eq!(
            Status::load(&dir).unwrap().unwrap().operation,
            OperationState::Rebase
        );
        fs::remove_dir_all(status.git_dir.join("rebase-merge")).unwrap();
        fs::write(status.git_dir.join("MERGE_HEAD"), "deadbeef\n").unwrap();
        assert_eq!(
            Status::load(&dir).unwrap().unwrap().operation,
            OperationState::Merge
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn loads_a_worktree_where_git_is_a_file() {
        let dir = repo("worktree");
        fs::write(dir.join("a.txt"), "one\n").unwrap();
        git(&dir, &["add", "a.txt"]);
        git(&dir, &["commit", "-qm", "one"]);
        let worktree = dir.join("linked");
        git(&dir, &["worktree", "add", "-q", worktree.to_str().unwrap()]);
        assert!(worktree.join(".git").is_file());
        let status = Status::load(&worktree).unwrap().unwrap();
        assert!(status.git_dir.join("HEAD").is_file());
        assert_eq!(
            status.branch_label(),
            git(&worktree, &["branch", "--show-current"])
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    #[cfg(unix)]
    fn preserves_non_utf8_paths() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        let dir = repo("non_utf8");
        let name = OsString::from_vec(vec![b'a', 0xff, b'b']);
        let path = dir.join(&name);
        fs::write(&path, "data\n").unwrap();
        let status = Status::load(&dir).unwrap().unwrap();
        let entry = status
            .entries
            .iter()
            .find(|entry| entry.kind == EntryKind::Untracked)
            .unwrap();
        assert_eq!(entry.path.as_os_str(), name.as_os_str());
        let _ = fs::remove_dir_all(dir);
    }
}
