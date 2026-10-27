use std::path::{Path, PathBuf};

use super::ExplorerState;

impl ExplorerState {
    pub(crate) fn select(&mut self, path: PathBuf) {
        self.selected = Some(path);
    }

    pub(crate) fn select_at(&mut self, index: usize, height: usize) {
        let Some(row) = self.rows.get(index) else {
            return;
        };
        self.selected = Some(row.path.clone());
        self.ensure_visible(height);
    }

    pub(crate) fn move_selection(&mut self, delta: isize, height: usize) {
        if self.rows.is_empty() {
            return;
        }
        let current = self.selected_index().unwrap_or(0);
        let next = if delta.is_negative() {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            current
                .saturating_add(delta.unsigned_abs())
                .min(self.rows.len().saturating_sub(1))
        };
        self.selected = Some(self.rows[next].path.clone());
        self.ensure_visible(height);
    }

    pub(crate) fn move_first(&mut self, height: usize) {
        if self.rows.is_empty() {
            return;
        }
        self.selected = Some(self.rows[0].path.clone());
        self.ensure_visible(height);
    }

    pub(crate) fn move_last(&mut self, height: usize) {
        let Some(row) = self.rows.last() else {
            return;
        };
        self.selected = Some(row.path.clone());
        self.ensure_visible(height);
    }

    pub(crate) fn ensure_visible(&mut self, height: usize) {
        let height = height.max(1);
        let Some(index) = self.selected_index() else {
            return;
        };
        if index < self.offset {
            self.offset = index;
        } else if index >= self.offset.saturating_add(height) {
            self.offset = index.saturating_add(1).saturating_sub(height);
        }
        self.offset = self.offset.min(self.rows.len().saturating_sub(height));
    }

    pub(crate) fn toggle_directory(&mut self, path: &Path) {
        if !self.expanded.insert(path.to_path_buf()) {
            self.expanded.remove(path);
        }
        self.selected = Some(path.to_path_buf());
        self.refresh_and_watch();
    }

    pub(crate) fn expand_directory(&mut self, path: &Path) {
        if self.expanded.insert(path.to_path_buf()) {
            self.refresh_and_watch();
        }
    }

    pub(crate) fn collapse_directory(&mut self, path: &Path) {
        if self.expanded.remove(path) {
            self.refresh_and_watch();
        }
    }

    pub(crate) fn reveal_path(&mut self, path: &Path) {
        let Some(root) = self.root.as_ref() else {
            return;
        };
        if !path.starts_with(root) {
            return;
        }
        self.expand_ancestors(path);
        self.selected = Some(path.to_path_buf());
        self.refresh_and_watch();
    }

    pub(crate) fn rename_path(&mut self, old: &Path, new: &Path) {
        self.expanded = self
            .expanded
            .iter()
            .map(|path| rewrite_path(path, old, new).unwrap_or_else(|| path.clone()))
            .collect();
        if let Some(selected) = self.selected.as_ref() {
            if let Some(path) = rewrite_path(selected, old, new) {
                self.selected = Some(path);
            }
        }
        self.refresh_and_watch();
    }

    pub(crate) fn remove_path(&mut self, target: &Path) {
        self.expanded.retain(|path| !path.starts_with(target));
        if self
            .selected
            .as_ref()
            .is_some_and(|path| path.starts_with(target))
        {
            self.selected = target.parent().map(Path::to_path_buf);
        }
        self.refresh_and_watch();
    }

    fn expand_ancestors(&mut self, path: &Path) {
        let Some(root) = self.root.as_deref() else {
            return;
        };
        let mut current = path.parent();
        while let Some(directory) = current {
            if !directory.starts_with(root) {
                break;
            }
            self.expanded.insert(directory.to_path_buf());
            if directory == root {
                break;
            }
            current = directory.parent();
        }
    }
}

fn rewrite_path(path: &Path, old: &Path, new: &Path) -> Option<PathBuf> {
    if path == old {
        return Some(new.to_path_buf());
    }
    let rest = path.strip_prefix(old).ok()?;
    (!rest.as_os_str().is_empty()).then(|| new.join(rest))
}
