use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

use super::fs::DirectoryFs;
use super::planner::{Operation, Plan};
use super::temporary::{unique_path, TemporaryPath};

const DELETE_MARKER: &str = "kawi-delete";

pub(super) struct Outcome {
    pub(super) completed: usize,
    pub(super) remaining: Vec<usize>,
    pub(super) rollback_errors: Vec<String>,
    pub(super) error: Option<String>,
}

impl Outcome {
    pub(super) fn failed(error: String) -> Self {
        Self {
            completed: 0,
            remaining: Vec::new(),
            rollback_errors: Vec::new(),
            error: Some(error),
        }
    }
}

enum Undo {
    Rename { from: PathBuf, to: PathBuf },
    Remove { path: PathBuf, kind: Removal },
    RestoreDelete(usize),
}

#[derive(Clone, Copy)]
enum Removal {
    File,
    Directory,
    Tree,
}

struct StagedDelete {
    operation: usize,
    original: PathBuf,
    staged: PathBuf,
    finalized: bool,
}

struct Execution<'a, F> {
    plan: &'a Plan,
    fs: &'a mut F,
    reserved: BTreeSet<PathBuf>,
    undo: Vec<Undo>,
    deletes: Vec<StagedDelete>,
    completed: usize,
}

pub(super) fn execute(plan: &Plan, fs: &mut impl DirectoryFs) -> Outcome {
    let mut execution = Execution::new(plan, fs);
    let error = execution.apply_plan();
    execution.finish(error)
}

impl<'a, F: DirectoryFs> Execution<'a, F> {
    fn new(plan: &'a Plan, fs: &'a mut F) -> Self {
        Self {
            plan,
            fs,
            reserved: reserved_paths(plan),
            undo: Vec::new(),
            deletes: Vec::new(),
            completed: 0,
        }
    }

    fn apply_plan(&mut self) -> Option<String> {
        for (index, operation) in self.plan.operations.iter().enumerate() {
            if let Err(error) = self.apply_operation(index, operation) {
                return Some(format!("{}: {error}", describe(operation)));
            }
            self.completed = self.completed.saturating_add(1);
        }
        self.finalize_deletes()
    }

    fn apply_operation(&mut self, index: usize, operation: &Operation) -> io::Result<()> {
        match operation {
            Operation::Rename { from, to } => {
                self.fs.rename(from, to)?;
                self.undo.push(Undo::Rename {
                    from: to.clone(),
                    to: from.clone(),
                });
            }
            Operation::Copy { from, to } => {
                self.fs.copy(from, to)?;
                self.undo.push(Undo::Remove {
                    path: to.clone(),
                    kind: Removal::Tree,
                });
            }
            Operation::CreateDirectory(path) => {
                self.fs.create_dir(path)?;
                self.undo.push(Undo::Remove {
                    path: path.clone(),
                    kind: Removal::Directory,
                });
            }
            Operation::CreateFile(path) => {
                self.fs.create_file(path)?;
                self.undo.push(Undo::Remove {
                    path: path.clone(),
                    kind: Removal::File,
                });
            }
            Operation::Trash(path) => self.stage_delete(index, path)?,
        }
        Ok(())
    }

    fn stage_delete(&mut self, index: usize, path: &Path) -> io::Result<()> {
        let parent = path.parent().unwrap_or_else(|| Path::new(""));
        let spec = TemporaryPath::new(path, parent, DELETE_MARKER, index);
        let staged = unique_path(&spec, &self.reserved, self.fs).map_err(io::Error::other)?;
        self.reserved.insert(staged.clone());
        self.fs.rename(path, &staged)?;
        let delete = self.deletes.len();
        self.deletes.push(StagedDelete {
            operation: index,
            original: path.to_path_buf(),
            staged,
            finalized: false,
        });
        self.undo.push(Undo::RestoreDelete(delete));
        Ok(())
    }

    fn finalize_deletes(&mut self) -> Option<String> {
        for index in 0..self.deletes.len() {
            let (staged, original) = {
                let delete = &self.deletes[index];
                (delete.staged.clone(), delete.original.clone())
            };
            if let Err(error) = self.fs.finish_delete(&staged, &original) {
                return Some(format!(
                    "{}: {error}",
                    describe(&Operation::Trash(original))
                ));
            }
            self.deletes[index].finalized = true;
        }
        None
    }

    fn finish(&mut self, error: Option<String>) -> Outcome {
        let completed = self.completed;
        let mut rollback_errors = Vec::new();
        let remaining = if error.is_some() {
            rollback_errors = self.rollback();
            self.deletes
                .iter()
                .filter(|delete| delete.finalized)
                .map(|delete| delete.operation)
                .collect()
        } else {
            (0..self.plan.operations.len()).collect()
        };
        Outcome {
            completed,
            remaining,
            rollback_errors,
            error,
        }
    }

    fn rollback(&mut self) -> Vec<String> {
        let mut errors = Vec::new();
        while let Some(undo) = self.undo.pop() {
            if let Err(error) = self.apply_undo(undo) {
                errors.push(error);
            }
        }
        errors
    }

    fn remove(&mut self, path: &Path, kind: Removal) -> io::Result<()> {
        match kind {
            Removal::File => self.fs.remove_file(path),
            Removal::Directory => self.fs.remove_directory(path),
            Removal::Tree => self.fs.remove_tree(path),
        }
    }

    fn apply_undo(&mut self, undo: Undo) -> Result<(), String> {
        match undo {
            Undo::Rename { from, to } => self
                .fs
                .rename(&from, &to)
                .map_err(|error| format!("cannot restore {}: {error}", to.display())),
            Undo::Remove { path, kind } => self
                .remove(&path, kind)
                .map_err(|error| format!("cannot remove {}: {error}", path.display())),
            Undo::RestoreDelete(index) => {
                let delete = &self.deletes[index];
                if delete.finalized {
                    return Ok(());
                }
                self.fs
                    .rename(&delete.staged, &delete.original)
                    .map_err(|error| {
                        format!("cannot restore {}: {error}", delete.original.display())
                    })
            }
        }
    }
}

fn reserved_paths(plan: &Plan) -> BTreeSet<PathBuf> {
    let mut paths = BTreeSet::new();
    for operation in &plan.operations {
        match operation {
            Operation::Rename { from, to } | Operation::Copy { from, to } => {
                paths.insert(from.clone());
                paths.insert(to.clone());
            }
            Operation::CreateDirectory(path)
            | Operation::CreateFile(path)
            | Operation::Trash(path) => {
                paths.insert(path.clone());
            }
        }
    }
    paths
}

fn describe(operation: &Operation) -> String {
    match operation {
        Operation::Rename { from, to } => {
            format!("cannot rename {} to {}", from.display(), to.display())
        }
        Operation::Copy { from, to } => {
            format!("cannot copy {} to {}", from.display(), to.display())
        }
        Operation::CreateDirectory(path) => {
            format!("cannot create directory {}", path.display())
        }
        Operation::CreateFile(path) => format!("cannot create {}", path.display()),
        Operation::Trash(path) => format!("cannot delete {}", path.display()),
    }
}

pub(super) fn applied_description(operation: &Operation) -> String {
    match operation {
        Operation::Rename { from, to } => {
            format!("renamed {} to {}", from.display(), to.display())
        }
        Operation::Copy { from, to } => {
            format!("copied {} to {}", from.display(), to.display())
        }
        Operation::CreateDirectory(path) => format!("created directory {}", path.display()),
        Operation::CreateFile(path) => format!("created {}", path.display()),
        Operation::Trash(path) => format!("removed {}", path.display()),
    }
}
