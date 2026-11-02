use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::super::fs::DirectoryFs;
use super::super::planner::{Operation, Plan};
use super::super::temporary::{unique_path, TemporaryPath};
use super::Execution;

const DELETE_MARKER: &str = "kawi-delete";

pub(super) struct StagedDelete {
    pub(super) operation: usize,
    pub(super) original: PathBuf,
    pub(super) staged: PathBuf,
    pub(super) finalized: bool,
}

pub(in crate::app::directory) struct PreparedPlan<'a> {
    plan: &'a Plan,
    deletes: Vec<StagedDelete>,
}

impl<'a> PreparedPlan<'a> {
    pub(in crate::app::directory) fn new(
        plan: &'a Plan,
        fs: &impl DirectoryFs,
    ) -> Result<Self, String> {
        let mut reserved = reserved_paths(plan);
        let mut deletes = Vec::new();
        for (index, operation) in plan.operations.iter().enumerate() {
            let Operation::Trash(path) = operation else {
                continue;
            };
            let staged = staged_path(path, index, &mut reserved, fs)?;
            deletes.push(StagedDelete {
                operation: index,
                original: path.clone(),
                staged,
                finalized: false,
            });
        }
        Ok(Self { plan, deletes })
    }

    pub(in crate::app::directory) fn plan(&self) -> &'a Plan {
        self.plan
    }

    pub(in crate::app::directory) fn staged_path(&self, operation: usize) -> Option<&Path> {
        self.deletes
            .iter()
            .find(|delete| delete.operation == operation)
            .map(|delete| delete.staged.as_path())
    }

    pub(in crate::app::directory) fn execute(self, fs: &mut impl DirectoryFs) -> super::Outcome {
        Execution::new(self.plan, fs, self.deletes).run()
    }
}

fn staged_path(
    path: &Path,
    index: usize,
    reserved: &mut BTreeSet<PathBuf>,
    fs: &impl DirectoryFs,
) -> Result<PathBuf, String> {
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let spec = TemporaryPath::new(path, parent, DELETE_MARKER, index);
    let staged = unique_path(&spec, reserved, fs)?;
    reserved.insert(staged.clone());
    Ok(staged)
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
