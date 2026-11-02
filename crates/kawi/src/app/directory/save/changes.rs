use std::path::PathBuf;

use super::super::diff::Change;
use super::super::session::DirectoryEdit;

pub(super) fn rename_requests(edits: &[DirectoryEdit]) -> Vec<(PathBuf, PathBuf)> {
    edits
        .iter()
        .flat_map(|edit| edit.changes.iter())
        .filter_map(|change| match change {
            Change::Rename { from, to, .. } => Some((from.clone(), to.clone())),
            _ => None,
        })
        .collect()
}

pub(in crate::app::directory) fn all_changes(edits: &[DirectoryEdit]) -> Vec<Change> {
    edits
        .iter()
        .flat_map(|edit| edit.changes.iter().cloned())
        .collect()
}

impl crate::app::App {
    pub(super) fn notify_lsp_changes(&mut self, changes: &[Change]) {
        let renames: Vec<(PathBuf, PathBuf)> = changes
            .iter()
            .filter_map(|change| match change {
                Change::Rename { from, to, .. } => Some((from.clone(), to.clone())),
                _ => None,
            })
            .collect();
        if !renames.is_empty() {
            self.lsp.did_rename_files(&renames);
        }
        let deletes: Vec<PathBuf> = changes
            .iter()
            .filter_map(|change| match change {
                Change::Delete { path, .. } => Some(path.clone()),
                _ => None,
            })
            .collect();
        if !deletes.is_empty() {
            self.lsp.did_delete_files(&deletes);
        }
    }
}
