use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::diff::Change;
use super::entry::EntryKind;
use super::fs::DirectoryFs;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Operation {
    Rename { from: PathBuf, to: PathBuf },
    Copy { from: PathBuf, to: PathBuf },
    CreateDirectory(PathBuf),
    CreateFile(PathBuf),
    Trash(PathBuf),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::app) struct Plan {
    pub(super) operations: Vec<Operation>,
    pub(super) changes: usize,
}

impl Plan {
    pub(super) fn needs_confirmation(&self) -> bool {
        self.changes > 1
            || self
                .operations
                .iter()
                .any(|operation| matches!(operation, Operation::Trash(_)))
    }
}

pub(super) fn plan(changes: Vec<Change>, fs: &impl DirectoryFs) -> Result<Plan, String> {
    let count = changes.len();
    let mut deletes = Vec::new();
    let mut renames = Vec::new();
    let mut copies = Vec::new();
    let mut files = Vec::new();
    let mut directories = BTreeSet::new();
    let mut vacated = BTreeSet::new();
    let mut targets = BTreeSet::new();
    for change in &changes {
        match change {
            Change::Delete { path, .. } => {
                vacated.insert(path.clone());
                deletes.push(path.clone());
            }
            Change::Rename { from, to, kind, .. } => {
                vacated.insert(from.clone());
                targets.insert(to.clone());
                if *kind == EntryKind::Directory && to.starts_with(from) {
                    return Err(format!("cannot move {} into itself", from.display()));
                }
                renames.push((from.clone(), to.clone()));
            }
            Change::Copy { to, .. } => {
                targets.insert(to.clone());
                copies.push(change.clone());
            }
            Change::Create { path, kind } => {
                targets.insert(path.clone());
                if *kind == EntryKind::Directory {
                    directories.insert(path.clone());
                } else {
                    files.push(path.clone());
                }
            }
        }
    }
    for target in &targets {
        if fs.exists(target) && !vacated.contains(target) {
            return Err(format!("target already exists: {}", target.display()));
        }
    }
    for (from, _) in &renames {
        if !fs.exists(from) {
            return Err(format!("source no longer exists: {}", from.display()));
        }
    }
    for path in &deletes {
        if !fs.exists(path) {
            return Err(format!("path no longer exists: {}", path.display()));
        }
    }
    for target in &targets {
        collect_missing_directories(target, fs, &mut directories)?;
    }
    for target in &targets {
        let parent = target.parent().unwrap_or_else(|| Path::new(""));
        let exists = parent.as_os_str().is_empty() || fs.is_dir(parent);
        if !exists && !directories.contains(parent) {
            return Err(format!(
                "parent directory does not exist: {}",
                parent.display()
            ));
        }
    }
    let mut operations = Vec::new();
    for path in deletes {
        operations.push(Operation::Trash(path));
    }
    let mut finals = Vec::new();
    for (index, (from, to)) in renames.into_iter().enumerate() {
        let temporary = temporary_path(&from, index, &targets, fs)?;
        operations.push(Operation::Rename {
            from: from.clone(),
            to: temporary.clone(),
        });
        finals.push(Operation::Rename {
            from: temporary,
            to: to.clone(),
        });
    }
    let mut directory_paths: Vec<PathBuf> = directories.into_iter().collect();
    directory_paths.sort_by_key(|path| path.components().count());
    operations.extend(directory_paths.into_iter().map(Operation::CreateDirectory));
    operations.extend(finals);
    operations.extend(files.into_iter().map(Operation::CreateFile));
    let mut renamed_by_id = BTreeMap::new();
    for change in &changes {
        if let Change::Rename { id, to, .. } = change {
            renamed_by_id.insert(*id, to.clone());
        }
    }
    for change in copies {
        let Change::Copy { id, from, to, .. } = change else {
            continue;
        };
        let source = renamed_by_id.get(&id).cloned().unwrap_or(from);
        operations.push(Operation::Copy { from: source, to });
    }
    Ok(Plan {
        operations,
        changes: count,
    })
}

fn collect_missing_directories(
    target: &Path,
    fs: &impl DirectoryFs,
    directories: &mut BTreeSet<PathBuf>,
) -> Result<(), String> {
    let mut current = target.parent();
    while let Some(path) = current {
        if path.as_os_str().is_empty() || fs.exists(path) {
            if !path.as_os_str().is_empty() && !fs.is_dir(path) {
                return Err(format!("parent is not a directory: {}", path.display()));
            }
            return Ok(());
        }
        directories.insert(path.to_path_buf());
        current = path.parent();
    }
    Ok(())
}

fn temporary_path(
    from: &Path,
    index: usize,
    targets: &BTreeSet<PathBuf>,
    fs: &impl DirectoryFs,
) -> Result<PathBuf, String> {
    let parent = from.parent().unwrap_or_else(|| Path::new(""));
    let name = from
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "item".to_string());
    for attempt in 0..10_000 {
        let candidate = parent.join(format!(".{name}.kawi-move-{index}-{attempt}"));
        if !fs.exists(&candidate) && !targets.contains(&candidate) {
            return Ok(candidate);
        }
    }
    Err(format!(
        "cannot find a temporary name for {}",
        from.display()
    ))
}
