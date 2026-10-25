use std::path::{Path, PathBuf};

use kawi_picker::PickerRow;

use super::super::App;
use super::state::GitBranchState;

impl App {
    pub(crate) fn git_root(&self) -> Option<PathBuf> {
        let start = if self.editor.has_file() {
            PathBuf::from(self.editor.filename())
        } else {
            std::env::current_dir().ok()?
        };
        kawi_git::repository_root(&start).ok().flatten()
    }

    pub(crate) fn git_unavailable(&mut self) -> bool {
        if self.git_root().is_some() {
            return false;
        }
        self.show_message("Not inside a git repository".to_string());
        true
    }
}

pub(crate) fn branch_row(branch: &kawi_git::Branch) -> PickerRow {
    PickerRow {
        display: branch_label(branch),
        path: branch.full_ref.clone(),
        line: None,
        column: None,
        buffer: None,
    }
}

fn branch_label(branch: &kawi_git::Branch) -> String {
    let marker = if branch.is_current { "* " } else { "  " };
    let mut label = format!("{marker}{}", branch.name);
    if branch.kind == kawi_git::BranchKind::Remote {
        label.push_str(" [remote]");
    }
    if let Some(upstream) = branch.upstream.as_deref() {
        label.push_str(" -> ");
        label.push_str(upstream);
    }
    if !branch.is_current {
        if let Some(path) = branch.worktree.as_deref() {
            label.push_str(" [worktree: ");
            label.push_str(&path.display().to_string());
            label.push(']');
        }
    }
    label
}

pub(crate) fn branch_by_ref<'a>(
    state: &'a GitBranchState,
    full_ref: &str,
) -> Option<&'a kawi_git::Branch> {
    state
        .branches
        .as_ref()?
        .branches
        .iter()
        .find(|branch| branch.full_ref == full_ref)
        .or_else(|| {
            state
                .branches
                .as_ref()?
                .detached
                .as_ref()
                .filter(|branch| branch.full_ref == full_ref)
        })
}

pub(crate) fn path_relative(root: &Path, path: &Path) -> Option<PathBuf> {
    path.strip_prefix(root).ok().map(Path::to_path_buf)
}
