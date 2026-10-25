use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use super::{BranchStatus, EntryKind, OperationState, Status, StatusCode, StatusEntry, Upstream};
use crate::command::path_from_bytes;

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

pub(super) fn detect_operation(git_dir: &Path) -> OperationState {
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
