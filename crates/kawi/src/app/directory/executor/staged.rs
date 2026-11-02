use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::super::fs::DirectoryFs;
use super::super::planner::{Operation, Plan};
use super::super::temporary::{unique_path, TemporaryPath};
use super::Execution;

const DELETE_MARKER: &str = "kawi-delete";
const STAGE_MARKER: &str = "kawi-copy";

pub(super) struct StagedDelete {
    pub(super) operation: usize,
    pub(super) original: PathBuf,
    pub(super) staged: PathBuf,
    pub(super) finalized: bool,
}

pub(in crate::app::directory) struct PreparedPlan<'a> {
    plan: &'a Plan,
    deletes: Vec<StagedDelete>,
    delete_positions: Vec<Option<usize>>,
    stages: Vec<Option<PathBuf>>,
}

impl<'a> PreparedPlan<'a> {
    pub(in crate::app::directory) fn new(
        plan: &'a Plan,
        fs: &impl DirectoryFs,
    ) -> Result<Self, String> {
        let mut reserved = reserved_paths(plan);
        let mut deletes = Vec::new();
        let mut delete_positions = vec![None; plan.operations.len()];
        let mut stages = vec![None; plan.operations.len()];
        for (index, operation) in plan.operations.iter().enumerate() {
            match operation {
                Operation::Trash(path) => {
                    let staged = staged_path(path, index, DELETE_MARKER, &mut reserved, fs)?;
                    if let Some(slot) = delete_positions.get_mut(index) {
                        *slot = Some(deletes.len());
                    }
                    deletes.push(StagedDelete {
                        operation: index,
                        original: path.clone(),
                        staged,
                        finalized: false,
                    });
                }
                Operation::Rename { from, to } if from.parent() != to.parent() => {
                    stages[index] = Some(staged_path(to, index, STAGE_MARKER, &mut reserved, fs)?);
                }
                Operation::Copy { to, .. } => {
                    stages[index] = Some(staged_path(to, index, STAGE_MARKER, &mut reserved, fs)?);
                }
                Operation::Rename { .. }
                | Operation::CreateDirectory(_)
                | Operation::CreateFile(_) => {}
            }
        }
        Ok(Self {
            plan,
            deletes,
            delete_positions,
            stages,
        })
    }

    pub(in crate::app::directory) fn plan(&self) -> &'a Plan {
        self.plan
    }

    pub(in crate::app::directory) fn staged_path(&self, operation: usize) -> Option<&Path> {
        let position = self.delete_positions.get(operation).copied().flatten()?;
        self.deletes
            .get(position)
            .map(|delete| delete.staged.as_path())
    }

    pub(in crate::app::directory) fn stage_path(&self, operation: usize) -> Option<&Path> {
        self.stages.get(operation).and_then(Option::as_deref)
    }

    pub(in crate::app::directory) fn execute(
        self,
        fs: &mut impl DirectoryFs,
        promotions: &mut dyn FnMut(usize) -> std::io::Result<()>,
    ) -> super::Outcome {
        Execution::new(
            self.plan,
            fs,
            self.deletes,
            self.delete_positions,
            self.stages,
            promotions,
        )
        .run()
    }
}

fn staged_path(
    path: &Path,
    index: usize,
    marker: &str,
    reserved: &mut BTreeSet<PathBuf>,
    fs: &impl DirectoryFs,
) -> Result<PathBuf, String> {
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let spec = TemporaryPath::new(path, parent, marker, index);
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
