use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::diff::Change;
use super::entry::EntryKind;
use super::fs::DirectoryFs;
use super::planner::Operation;

type Operations = BTreeSet<String>;

#[derive(Default)]
struct Requirements {
    writable: BTreeMap<PathBuf, Operations>,
    directory_writes: BTreeMap<PathBuf, Operations>,
    readable: BTreeMap<PathBuf, Operations>,
    removable_entries: BTreeMap<PathBuf, Operations>,
    deletes: BTreeMap<PathBuf, Operations>,
}

pub(super) fn preflight(
    changes: &[Change],
    operations: &[Operation],
    fs: &impl DirectoryFs,
) -> Result<(), String> {
    let requirements = Requirements::collect(changes, operations, fs)?;
    let failures = requirements.failures(fs);
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("\n"))
    }
}

impl Requirements {
    fn collect(
        changes: &[Change],
        operations: &[Operation],
        fs: &impl DirectoryFs,
    ) -> Result<Self, String> {
        let mut requirements = Self::default();
        requirements.add_changes(changes, fs)?;
        requirements.add_operations(operations, fs)?;
        Ok(requirements)
    }

    fn add_changes(&mut self, changes: &[Change], fs: &impl DirectoryFs) -> Result<(), String> {
        for change in changes {
            let operation = change_label(change);
            match change {
                Change::Rename { from, to, kind, .. } => {
                    self.add_writable_parent(from, &operation, fs)?;
                    self.add_writable_parent(to, &operation, fs)?;
                    add(&mut self.removable_entries, from, &operation);
                    if *kind == EntryKind::Directory && from.parent() != to.parent() {
                        add(&mut self.directory_writes, from, &operation);
                    }
                }
                Change::Copy { from, to, .. } => {
                    add(&mut self.readable, from, &operation);
                    self.add_writable_parent(to, &operation, fs)?;
                }
                Change::Create { path, .. } => {
                    self.add_writable_parent(path, &operation, fs)?;
                }
                Change::Delete { path, .. } => {
                    self.add_writable_parent(path, &operation, fs)?;
                    add(&mut self.removable_entries, path, &operation);
                }
            }
        }
        Ok(())
    }

    fn add_operations(
        &mut self,
        operations: &[Operation],
        fs: &impl DirectoryFs,
    ) -> Result<(), String> {
        for operation in operations {
            let label = operation_label(operation);
            match operation {
                Operation::Rename { from, to, .. } => {
                    self.add_missing_writable_parent(from, &label, fs)?;
                    self.add_missing_writable_parent(to, &label, fs)?;
                }
                Operation::Copy { to, .. }
                | Operation::CreateDirectory(to)
                | Operation::CreateFile(to) => {
                    self.add_missing_writable_parent(to, &label, fs)?;
                }
                Operation::Trash(path) => {
                    self.add_missing_writable_parent(path, &label, fs)?;
                    add(&mut self.deletes, path, &label);
                }
            }
        }
        Ok(())
    }

    fn add_writable_parent(
        &mut self,
        path: &Path,
        operation: &str,
        fs: &impl DirectoryFs,
    ) -> Result<(), String> {
        let parent = existing_parent(path, fs)?;
        add(&mut self.writable, &parent, operation);
        Ok(())
    }

    fn add_missing_writable_parent(
        &mut self,
        path: &Path,
        operation: &str,
        fs: &impl DirectoryFs,
    ) -> Result<(), String> {
        let parent = existing_parent(path, fs)?;
        if !self.writable.contains_key(&parent) {
            add(&mut self.writable, &parent, operation);
        }
        Ok(())
    }

    fn failures(&self, fs: &impl DirectoryFs) -> Vec<String> {
        let mut failures = Vec::new();
        check_writable(&self.writable, fs, &mut failures);
        check_directory_writes(&self.directory_writes, fs, &mut failures);
        check_readable(&self.readable, fs, &mut failures);
        check_removable_entries(&self.removable_entries, fs, &mut failures);
        check_deletes(&self.deletes, fs, &mut failures);
        failures
    }
}

fn check_writable(
    paths: &BTreeMap<PathBuf, Operations>,
    fs: &impl DirectoryFs,
    failures: &mut Vec<String>,
) {
    for (path, operations) in paths {
        if !fs.can_modify_directory(path) {
            failures.push(format!(
                "{}: parent {} requires write and execute permission",
                operation_list(operations),
                path.display()
            ));
        }
    }
}

fn check_directory_writes(
    paths: &BTreeMap<PathBuf, Operations>,
    fs: &impl DirectoryFs,
    failures: &mut Vec<String>,
) {
    for (path, operations) in paths {
        if !fs.can_write_directory(path) {
            failures.push(format!(
                "{}: directory {} requires write permission",
                operation_list(operations),
                path.display()
            ));
        }
    }
}

fn check_readable(
    paths: &BTreeMap<PathBuf, Operations>,
    fs: &impl DirectoryFs,
    failures: &mut Vec<String>,
) {
    for (path, operations) in paths {
        if !fs.can_read(path) {
            failures.push(format!(
                "{}: source {} requires read permission",
                operation_list(operations),
                path.display()
            ));
        }
    }
}

fn check_removable_entries(
    paths: &BTreeMap<PathBuf, Operations>,
    fs: &impl DirectoryFs,
    failures: &mut Vec<String>,
) {
    for (path, operations) in paths {
        if !fs.can_remove_entry(path) {
            let parent = path.parent().unwrap_or_else(|| Path::new(""));
            failures.push(format!(
                "{}: sticky directory {} restricts removal to the entry owner or directory owner",
                operation_list(operations),
                parent.display()
            ));
        }
    }
}

fn check_deletes(
    paths: &BTreeMap<PathBuf, Operations>,
    fs: &impl DirectoryFs,
    failures: &mut Vec<String>,
) {
    for (path, operations) in paths {
        if let Some(reason) = fs.delete_error(path) {
            failures.push(format!("{}: {reason}", operation_list(operations)));
        }
    }
}

fn add(paths: &mut BTreeMap<PathBuf, Operations>, path: &Path, operation: &str) {
    paths
        .entry(path.to_path_buf())
        .or_default()
        .insert(operation.to_string());
}

fn operation_list(operations: &Operations) -> String {
    operations.iter().cloned().collect::<Vec<_>>().join(", ")
}

fn change_label(change: &Change) -> String {
    match change {
        Change::Rename { from, to, .. } => {
            format!("move {} to {}", from.display(), to.display())
        }
        Change::Copy { from, to, .. } => {
            format!("copy {} to {}", from.display(), to.display())
        }
        Change::Create { path, kind } => {
            if *kind == EntryKind::Directory {
                format!("create directory {}", path.display())
            } else {
                format!("create {}", path.display())
            }
        }
        Change::Delete { path, .. } => format!("delete {}", path.display()),
    }
}

fn operation_label(operation: &Operation) -> String {
    match operation {
        Operation::Rename { from, to, .. } => {
            format!("move {} to {}", from.display(), to.display())
        }
        Operation::Copy { from, to } => {
            format!("copy {} to {}", from.display(), to.display())
        }
        Operation::CreateDirectory(path) => format!("create directory {}", path.display()),
        Operation::CreateFile(path) => format!("create {}", path.display()),
        Operation::Trash(path) => format!("delete {}", path.display()),
    }
}

fn existing_parent(path: &Path, fs: &impl DirectoryFs) -> Result<PathBuf, String> {
    let mut parent = path
        .parent()
        .ok_or_else(|| format!("cannot check permission for {}", path.display()))?;
    loop {
        if parent.as_os_str().is_empty() {
            return Err(format!("cannot check permission for {}", path.display()));
        }
        if fs.exists(parent) {
            if fs.is_dir(parent) {
                return Ok(parent.to_path_buf());
            }
            return Err(format!("parent is not a directory: {}", parent.display()));
        }
        parent = parent
            .parent()
            .ok_or_else(|| format!("cannot check permission for {}", path.display()))?;
    }
}
