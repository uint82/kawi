use std::io;
use std::path::{Path, PathBuf};

use super::fs::DirectoryFs;
use super::planner::{Operation, Plan};
use staged::StagedDelete;

mod describe;
mod staged;

pub(in crate::app::directory) use describe::applied_description;
use describe::describe;
pub(in crate::app) use staged::PreparedPlan;

pub(in crate::app) struct Outcome {
    pub(in crate::app) completed: usize,
    pub(in crate::app) remaining: Vec<usize>,
    pub(in crate::app) rollback_errors: Vec<String>,
    pub(in crate::app) deferred_deletes: Vec<PathBuf>,
    pub(in crate::app) error: Option<String>,
}

impl Outcome {
    pub(super) fn failed(error: String) -> Self {
        Self {
            completed: 0,
            remaining: Vec::new(),
            rollback_errors: Vec::new(),
            deferred_deletes: Vec::new(),
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

struct Execution<'a, F> {
    plan: &'a Plan,
    fs: &'a mut F,
    undo: Vec<Undo>,
    deletes: Vec<StagedDelete>,
    delete_positions: Vec<Option<usize>>,
    stages: Vec<Option<PathBuf>>,
    promotions: &'a mut dyn FnMut(usize) -> io::Result<()>,
    deferred_deletes: Vec<PathBuf>,
    completed: usize,
}

#[cfg(test)]
pub(super) fn execute(plan: &Plan, fs: &mut impl DirectoryFs) -> Outcome {
    match PreparedPlan::new(plan, fs) {
        Ok(prepared) => prepared.execute(fs, &mut |_| Ok(())),
        Err(error) => Outcome::failed(error),
    }
}

impl<'a, F: DirectoryFs> Execution<'a, F> {
    fn new(
        plan: &'a Plan,
        fs: &'a mut F,
        deletes: Vec<StagedDelete>,
        delete_positions: Vec<Option<usize>>,
        stages: Vec<Option<PathBuf>>,
        promotions: &'a mut dyn FnMut(usize) -> io::Result<()>,
    ) -> Self {
        Self {
            plan,
            fs,
            undo: Vec::with_capacity(plan.operations.len()),
            deletes,
            delete_positions,
            stages,
            promotions,
            deferred_deletes: Vec::new(),
            completed: 0,
        }
    }

    fn run(mut self) -> Outcome {
        let error = self.apply_plan();
        self.finish(error)
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
            Operation::Rename { from, to, force } => self.rename(index, from, to, *force)?,
            Operation::Copy { from, to } => self.copy(index, from, to)?,
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
            Operation::Trash(_) => self.stage_delete(index)?,
        }
        Ok(())
    }

    fn rename(&mut self, operation: usize, from: &Path, to: &Path, force: bool) -> io::Result<()> {
        match self.fs.try_rename(from, to, force) {
            Ok(()) => {
                self.undo.push(Undo::Rename {
                    from: to.to_path_buf(),
                    to: from.to_path_buf(),
                });
                Ok(())
            }
            Err(error) if error.kind() == io::ErrorKind::CrossesDevices => {
                let stage = self.stage(operation)?;
                self.cross_device_rename(operation, from, to, force, &stage)
            }
            Err(error) => Err(error),
        }
    }

    fn cross_device_rename(
        &mut self,
        operation: usize,
        from: &Path,
        to: &Path,
        force: bool,
        stage: &Path,
    ) -> io::Result<()> {
        self.stage_copy(from, stage)?;
        (self.promotions)(operation)?;
        self.fs.promote_stage(stage, to, force)?;
        self.undo.push(Undo::Remove {
            path: to.to_path_buf(),
            kind: Removal::Tree,
        });
        self.deferred_deletes.push(from.to_path_buf());
        Ok(())
    }

    fn copy(&mut self, operation: usize, from: &Path, to: &Path) -> io::Result<()> {
        let stage = self.stage(operation)?;
        self.stage_copy(from, &stage)?;
        self.fs.promote_stage(&stage, to, false)?;
        self.undo.push(Undo::Remove {
            path: to.to_path_buf(),
            kind: Removal::Tree,
        });
        Ok(())
    }

    fn stage(&self, operation: usize) -> io::Result<PathBuf> {
        self.stages
            .get(operation)
            .cloned()
            .flatten()
            .ok_or_else(|| io::Error::other("copy stage was not prepared"))
    }

    fn stage_copy(&mut self, from: &Path, stage: &Path) -> io::Result<()> {
        self.fs.stage_copy(from, stage)?;
        self.undo.push(Undo::Remove {
            path: stage.to_path_buf(),
            kind: Removal::Tree,
        });
        Ok(())
    }

    fn stage_delete(&mut self, operation: usize) -> io::Result<()> {
        let position = self
            .delete_positions
            .get(operation)
            .copied()
            .flatten()
            .ok_or_else(|| io::Error::other("delete was not prepared"))?;
        let (original, staged) = {
            let delete = &self.deletes[position];
            (delete.original.clone(), delete.staged.clone())
        };
        self.fs.rename(&original, &staged)?;
        self.undo.push(Undo::RestoreDelete(position));
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
        let (remaining, deferred_deletes) = if error.is_some() {
            rollback_errors = self.rollback();
            let remaining = self
                .deletes
                .iter()
                .filter(|delete| delete.finalized)
                .map(|delete| delete.operation)
                .collect();
            (remaining, Vec::new())
        } else {
            (
                (0..self.plan.operations.len()).collect(),
                std::mem::take(&mut self.deferred_deletes),
            )
        };
        Outcome {
            completed,
            remaining,
            rollback_errors,
            deferred_deletes,
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
