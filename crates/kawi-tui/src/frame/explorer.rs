use crate::frame::{DiagnosticSeverity, Rect};

pub struct ExplorerPanel<'a> {
    pub rect: Rect,
    pub title: &'a str,
    pub status: &'a str,
    pub focused: bool,
    pub rows: &'a [ExplorerRow<'a>],
    pub selected: Option<usize>,
    pub offset: usize,
    pub error: Option<&'a str>,
}

pub struct ExplorerRow<'a> {
    pub depth: usize,
    pub name: &'a str,
    pub kind: ExplorerRowKind,
    pub hidden: bool,
    pub active: bool,
    pub dirty: bool,
    pub git: Option<ExplorerGit>,
    pub diagnostic: Option<DiagnosticSeverity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorerRowKind {
    Directory { expanded: bool },
    File,
    Symlink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExplorerGit {
    pub code: char,
    pub kind: ExplorerGitKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorerGitKind {
    Add,
    Change,
    Delete,
    Conflict,
    Untracked,
}
