use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::diff::Change;
use super::fs::DirectoryFs;
use super::planner::Operation;

#[derive(Default)]
struct Requirements {
    writable: BTreeSet<PathBuf>,
    readable: BTreeSet<PathBuf>,
    removable: BTreeSet<PathBuf>,
}

pub(super) fn preflight(
    changes: &[Change],
    operations: &[Operation],
    fs: &impl DirectoryFs,
) -> Result<(), String> {
    Requirements::collect(changes, operations, fs)?.check(fs)
}

impl Requirements {
    fn collect(
        changes: &[Change],
        operations: &[Operation],
        fs: &impl DirectoryFs,
    ) -> Result<Self, String> {
        let mut requirements = Self::default();
        for change in changes {
            if let Change::Copy { from, .. } = change {
                requirements.readable.insert(from.clone());
            }
        }
        for operation in operations {
            requirements.add_operation(operation, fs)?;
        }
        Ok(requirements)
    }

    fn add_operation(
        &mut self,
        operation: &Operation,
        fs: &impl DirectoryFs,
    ) -> Result<(), String> {
        match operation {
            Operation::Rename { from, to } => {
                self.add_writable_parent(from, fs)?;
                self.add_writable_parent(to, fs)?;
            }
            Operation::Copy { to, .. } => {
                self.add_writable_parent(to, fs)?;
            }
            Operation::CreateDirectory(path) | Operation::CreateFile(path) => {
                self.add_writable_parent(path, fs)?;
            }
            Operation::Trash(path) => {
                self.add_writable_parent(path, fs)?;
                self.removable.insert(path.clone());
            }
        }
        Ok(())
    }

    fn add_writable_parent(&mut self, path: &Path, fs: &impl DirectoryFs) -> Result<(), String> {
        self.writable.insert(existing_parent(path, fs)?);
        Ok(())
    }

    fn check(&self, fs: &impl DirectoryFs) -> Result<(), String> {
        for directory in &self.writable {
            if !fs.can_modify_directory(directory) {
                return Err(format!(
                    "Cannot modify {}: permission denied",
                    directory.display()
                ));
            }
        }
        for path in &self.readable {
            if !fs.can_read(path) {
                return Err(format!("Cannot read {}: permission denied", path.display()));
            }
        }
        for path in &self.removable {
            if !fs.can_delete(path) {
                return Err(format!(
                    "Cannot delete {}: permission denied or trash unavailable",
                    path.display()
                ));
            }
        }
        Ok(())
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
