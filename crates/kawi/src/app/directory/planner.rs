use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::diff::Change;
use super::entry::EntryKind;
use super::fs::DirectoryFs;
use super::temporary::{unique_path, TemporaryPath};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::app) enum Operation {
    Rename {
        from: PathBuf,
        to: PathBuf,
        force: bool,
    },
    Copy {
        from: PathBuf,
        to: PathBuf,
    },
    CreateDirectory(PathBuf),
    CreateFile(PathBuf),
    Trash(PathBuf),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::app) struct Plan {
    pub(in crate::app) operations: Vec<Operation>,
    pub(in crate::app) changes: usize,
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
    super::permissions::preflight(&changes, &[], fs)?;
    let mut deletes = Vec::new();
    let mut renames = Vec::new();
    let mut directory_renames = Vec::new();
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
                if *kind == EntryKind::Directory {
                    if to.starts_with(from) {
                        return Err(format!("cannot move {} into itself", from.display()));
                    }
                    directory_renames.push((from.clone(), to.clone()));
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
    validate_directory_rename_targets(&targets, &directory_renames)?;
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
    let renamed_directories: BTreeSet<PathBuf> = directory_renames
        .iter()
        .map(|(from, _)| from.clone())
        .collect();
    let temporary_names = TemporaryNames {
        targets: &targets,
        renamed_directories: &renamed_directories,
        fs,
    };
    let mut operations = Vec::new();
    for path in deletes {
        operations.push(Operation::Trash(path));
    }
    let mut finals = Vec::new();
    renames.sort_by_key(|(from, _)| std::cmp::Reverse(from.components().count()));
    for (index, (from, to)) in renames.into_iter().enumerate() {
        let temporary = temporary_names.path(&from, index)?;
        operations.push(Operation::Rename {
            force: false,
            from: from.clone(),
            to: temporary.clone(),
        });
        finals.push(Operation::Rename {
            force: false,
            from: temporary,
            to: to.clone(),
        });
    }
    let mut directory_paths: Vec<PathBuf> = directories.into_iter().collect();
    directory_paths.sort_by_key(|path| path.components().count());
    operations.extend(directory_paths.into_iter().map(Operation::CreateDirectory));
    operations.extend(finals);
    operations.extend(files.into_iter().map(Operation::CreateFile));
    let mut renamed_paths = BTreeMap::new();
    for change in &changes {
        if let Change::Rename { from, to, .. } = change {
            renamed_paths.insert(from.clone(), to.clone());
        }
    }
    for change in copies {
        let Change::Copy { from, to, .. } = change else {
            continue;
        };
        let source = renamed_paths.get(&from).unwrap_or(&from).clone();
        operations.push(Operation::Copy { from: source, to });
    }
    Ok(Plan {
        operations,
        changes: count,
    })
}

fn validate_directory_rename_targets(
    targets: &BTreeSet<PathBuf>,
    directory_renames: &[(PathBuf, PathBuf)],
) -> Result<(), String> {
    for (from, to) in directory_renames {
        for target in targets {
            let moves_under_source = target != from && target.starts_with(from);
            let moves_under_target = target != to && target.starts_with(to);
            if moves_under_source || moves_under_target {
                return Err(format!(
                    "cannot place {} inside directory being renamed from {} to {}",
                    target.display(),
                    from.display(),
                    to.display()
                ));
            }
        }
    }
    Ok(())
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

struct TemporaryNames<'a, F> {
    targets: &'a BTreeSet<PathBuf>,
    renamed_directories: &'a BTreeSet<PathBuf>,
    fs: &'a F,
}

impl<F: DirectoryFs> TemporaryNames<'_, F> {
    fn path(&self, from: &Path, index: usize) -> Result<PathBuf, String> {
        let parent = self.parent(from);
        let spec = TemporaryPath::new(from, &parent, "kawi-move", index);
        unique_path(&spec, self.targets, self.fs)
    }

    fn parent(&self, from: &Path) -> PathBuf {
        let mut parent = from.parent().unwrap_or_else(|| Path::new("")).to_path_buf();
        for ancestor in from.ancestors() {
            if self.renamed_directories.contains(ancestor) {
                parent = ancestor
                    .parent()
                    .unwrap_or_else(|| Path::new(""))
                    .to_path_buf();
            }
        }
        parent
    }
}
