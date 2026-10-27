use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kawi_git::{EntryKind, Status, StatusCode, StatusEntry};
use kawi_tui::frame::{DiagnosticSeverity, ExplorerGit, ExplorerGitKind};

use super::super::diagnostic_index::DiagnosticCounts;
use super::super::problems::{ProblemSeverity, Problems};
use super::ExplorerState;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct MarksKey {
    pub(super) problems: u64,
    pub(super) git: u64,
}

#[derive(Default)]
pub(super) struct ExplorerMarks {
    git_files: BTreeMap<PathBuf, ExplorerGit>,
    git_directories: BTreeMap<PathBuf, ExplorerGit>,
    diagnostic_files: BTreeMap<PathBuf, DiagnosticCounts>,
    diagnostic_directories: BTreeMap<PathBuf, DiagnosticCounts>,
}

impl ExplorerState {
    pub(crate) fn sync_marks(
        &mut self,
        root: &Path,
        problems: &Problems,
        status: Option<&Status>,
        git_revision: u64,
    ) {
        let key = MarksKey {
            problems: problems.revision(),
            git: git_revision,
        };
        if self.marks_root.as_deref() == Some(root) && self.marks_key == key {
            return;
        }
        self.marks = ExplorerMarks::rebuild(root, problems, status);
        self.marks_key = key;
        self.marks_root = Some(root.to_path_buf());
    }

    pub(crate) fn git(&self, path: &Path, directory: bool) -> Option<ExplorerGit> {
        self.marks.git(path, directory)
    }

    pub(crate) fn diagnostic(&self, path: &Path, directory: bool) -> Option<DiagnosticSeverity> {
        self.marks.diagnostic(path, directory)
    }
}

impl ExplorerMarks {
    pub(super) fn rebuild(root: &Path, problems: &Problems, status: Option<&Status>) -> Self {
        let mut marks = Self::default();
        marks.add_diagnostics(root, problems.path_counts());
        if let Some(status) = status {
            marks.add_git(root, status);
        }
        marks
    }

    pub(super) fn git(&self, path: &Path, directory: bool) -> Option<ExplorerGit> {
        if directory {
            self.git_directories.get(path).copied()
        } else {
            self.git_files.get(path).copied()
        }
    }

    pub(super) fn diagnostic(&self, path: &Path, directory: bool) -> Option<DiagnosticSeverity> {
        let counts = if directory {
            self.diagnostic_directories.get(path)?
        } else {
            self.diagnostic_files.get(path)?
        };
        counts.highest().map(ui_severity)
    }

    fn add_diagnostics(&mut self, root: &Path, counts: BTreeMap<PathBuf, DiagnosticCounts>) {
        for (path, counts) in counts {
            self.diagnostic_files.insert(path.clone(), counts);
            for directory in ancestor_directories(root, &path) {
                self.diagnostic_directories
                    .entry(directory)
                    .or_default()
                    .merge(counts);
            }
        }
    }

    fn add_git(&mut self, root: &Path, status: &Status) {
        for entry in &status.entries {
            let path = absolute_path(&status.root, &entry.path);
            let mark = git_mark(entry);
            self.git_files.insert(path.clone(), mark);
            for directory in ancestor_directories(root, &path) {
                let current = self.git_directories.entry(directory).or_insert(mark);
                if git_priority(mark.kind) > git_priority(current.kind) {
                    *current = mark;
                }
            }
        }
    }
}

fn absolute_path(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn ancestor_directories(root: &Path, path: &Path) -> Vec<PathBuf> {
    let mut directories = Vec::new();
    let mut current = path.parent();
    while let Some(directory) = current {
        if !directory.starts_with(root) || directory == root {
            break;
        }
        directories.push(directory.to_path_buf());
        current = directory.parent();
    }
    directories
}

fn git_mark(entry: &StatusEntry) -> ExplorerGit {
    if entry.kind == EntryKind::Unmerged {
        return ExplorerGit {
            code: '!',
            kind: ExplorerGitKind::Conflict,
        };
    }
    if entry.kind == EntryKind::Untracked {
        return ExplorerGit {
            code: '?',
            kind: ExplorerGitKind::Untracked,
        };
    }
    let code = if entry.worktree != StatusCode::Unmodified {
        entry.worktree
    } else {
        entry.index
    };
    let kind = match code {
        StatusCode::Added | StatusCode::Copied => ExplorerGitKind::Add,
        StatusCode::Deleted => ExplorerGitKind::Delete,
        StatusCode::Modified
        | StatusCode::Renamed
        | StatusCode::TypeChanged
        | StatusCode::Unmerged => ExplorerGitKind::Change,
        StatusCode::Untracked => ExplorerGitKind::Untracked,
        StatusCode::Unmodified => ExplorerGitKind::Change,
    };
    ExplorerGit {
        code: code.label(),
        kind,
    }
}

fn git_priority(kind: ExplorerGitKind) -> u8 {
    match kind {
        ExplorerGitKind::Untracked => 0,
        ExplorerGitKind::Add => 1,
        ExplorerGitKind::Change => 2,
        ExplorerGitKind::Delete => 3,
        ExplorerGitKind::Conflict => 4,
    }
}

fn ui_severity(severity: ProblemSeverity) -> DiagnosticSeverity {
    match severity {
        ProblemSeverity::Error => DiagnosticSeverity::Error,
        ProblemSeverity::Warning => DiagnosticSeverity::Warning,
        ProblemSeverity::Information => DiagnosticSeverity::Information,
        ProblemSeverity::Hint => DiagnosticSeverity::Hint,
    }
}

#[cfg(test)]
mod tests;
