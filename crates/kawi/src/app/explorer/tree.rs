use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::ExplorerRowKind;

pub(super) const MAX_EXPLORER_ROWS: usize = 10_000;
pub(super) const MAX_EXPLORER_DEPTH: usize = 64;

pub(super) struct TreeLoad {
    pub(super) rows: Vec<TreeRow>,
    pub(super) error: Option<String>,
    pub(super) truncated: bool,
}

pub(super) struct TreeRow {
    pub(super) path: PathBuf,
    pub(super) name: String,
    pub(super) depth: usize,
    pub(super) kind: ExplorerRowKind,
    pub(super) hidden: bool,
}

struct Child {
    path: PathBuf,
    name: String,
    sort_name: String,
    kind: ExplorerRowKind,
    hidden: bool,
}

struct TreeLoader<'a> {
    expanded: &'a BTreeSet<PathBuf>,
    show_hidden: bool,
    load: TreeLoad,
}

pub(super) fn load_rows(root: &Path, expanded: &BTreeSet<PathBuf>, show_hidden: bool) -> TreeLoad {
    TreeLoader {
        expanded,
        show_hidden,
        load: TreeLoad {
            rows: Vec::new(),
            error: None,
            truncated: false,
        },
    }
    .load(root)
}

impl TreeLoader<'_> {
    fn load(mut self, root: &Path) -> TreeLoad {
        self.load_children(root, 0);
        self.load
    }

    fn load_children(&mut self, directory: &Path, depth: usize) {
        if depth > MAX_EXPLORER_DEPTH {
            return;
        }
        if depth > 0 && !self.expanded.contains(directory) {
            return;
        }
        let children = match read_children(directory, self.show_hidden) {
            Ok(children) => children,
            Err(error) => {
                self.record_error(directory, error);
                return;
            }
        };
        for child in children {
            if self.load.rows.len() >= MAX_EXPLORER_ROWS {
                self.load.truncated = true;
                return;
            }
            let path = child.path.clone();
            let is_directory = matches!(child.kind, ExplorerRowKind::Directory { .. });
            self.load.rows.push(TreeRow {
                path: child.path,
                name: child.name,
                depth,
                kind: row_kind(child.kind, self.expanded, &path),
                hidden: child.hidden,
            });
            if is_directory {
                self.load_children(&path, depth.saturating_add(1));
            }
        }
    }

    fn record_error(&mut self, directory: &Path, error: io::Error) {
        if self.load.error.is_none() {
            self.load.error = Some(format!("Cannot read {}: {error}", directory.display()));
        }
    }
}

fn row_kind(kind: ExplorerRowKind, expanded: &BTreeSet<PathBuf>, path: &Path) -> ExplorerRowKind {
    match kind {
        ExplorerRowKind::Directory { .. } => ExplorerRowKind::Directory {
            expanded: expanded.contains(path),
        },
        other => other,
    }
}

fn read_children(directory: &Path, show_hidden: bool) -> io::Result<Vec<Child>> {
    let mut children = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let hidden = name.starts_with('.');
        let file_type = entry.file_type()?;
        let is_directory = file_type.is_dir() && !file_type.is_symlink();
        if is_directory && is_vcs_directory(&name) {
            continue;
        }
        if hidden && !show_hidden {
            continue;
        }
        let kind = entry_kind(file_type, is_directory);
        children.push(Child {
            path: entry.path(),
            sort_name: name.to_lowercase(),
            name,
            kind,
            hidden,
        });
    }
    children.sort_by(compare_children);
    Ok(children)
}

fn entry_kind(file_type: fs::FileType, is_directory: bool) -> ExplorerRowKind {
    if is_directory {
        ExplorerRowKind::Directory { expanded: false }
    } else if file_type.is_symlink() {
        ExplorerRowKind::Symlink
    } else {
        ExplorerRowKind::File
    }
}

fn compare_children(left: &Child, right: &Child) -> std::cmp::Ordering {
    let left_directory = matches!(left.kind, ExplorerRowKind::Directory { .. });
    let right_directory = matches!(right.kind, ExplorerRowKind::Directory { .. });
    right_directory
        .cmp(&left_directory)
        .then_with(|| left.sort_name.cmp(&right.sort_name))
        .then_with(|| left.name.cmp(&right.name))
}

fn is_vcs_directory(name: &str) -> bool {
    matches!(name, ".git" | ".hg" | ".svn")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;

    fn names(rows: &[TreeRow]) -> Vec<&str> {
        rows.iter().map(|row| row.name.as_str()).collect()
    }

    #[test]
    fn directories_sort_before_files_and_hidden_entries_are_filtered() {
        let dir = temp_dir("explorer_sort");
        fs::create_dir_all(dir.join("zeta")).unwrap();
        fs::write(dir.join("alpha.txt"), "").unwrap();
        fs::write(dir.join(".hidden"), "").unwrap();
        fs::write(dir.join("beta.txt"), "").unwrap();
        let load = load_rows(&dir, &BTreeSet::new(), false);
        assert!(load.error.is_none());
        assert_eq!(names(&load.rows), ["zeta", "alpha.txt", "beta.txt"]);
        let load = load_rows(&dir, &BTreeSet::new(), true);
        assert_eq!(
            names(&load.rows),
            ["zeta", ".hidden", "alpha.txt", "beta.txt"]
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn expanded_directories_include_children() {
        let dir = temp_dir("explorer_expand");
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src/main.rs"), "").unwrap();
        let mut expanded = BTreeSet::new();
        expanded.insert(dir.join("src"));
        let load = load_rows(&dir, &expanded, false);
        assert_eq!(names(&load.rows), ["src", "main.rs"]);
        assert_eq!(load.rows[1].depth, 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn collapsed_directories_do_not_include_children() {
        let dir = temp_dir("explorer_collapse");
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src/main.rs"), "").unwrap();
        let load = load_rows(&dir, &BTreeSet::new(), false);
        assert_eq!(names(&load.rows), ["src"]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn version_control_directories_are_always_hidden() {
        let dir = temp_dir("explorer_vcs");
        fs::create_dir_all(dir.join(".git")).unwrap();
        let load = load_rows(&dir, &BTreeSet::new(), true);
        assert!(load.rows.is_empty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_root_reports_an_error() {
        let dir = temp_dir("explorer_missing");
        let missing = dir.join("missing");
        let load = load_rows(&missing, &BTreeSet::new(), false);
        assert!(load.error.is_some());
        assert!(load.rows.is_empty());
        fs::remove_dir_all(dir).unwrap();
    }
}
