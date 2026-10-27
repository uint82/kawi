use super::fs::DirectoryFs;
use super::planner::{Operation, Plan};

pub(super) struct Outcome {
    pub(super) applied: usize,
    pub(super) error: Option<String>,
}

pub(super) fn execute(plan: &Plan, fs: &mut impl DirectoryFs) -> Outcome {
    let mut applied = 0;
    for operation in &plan.operations {
        let result = match operation {
            Operation::Rename { from, to } => fs.rename(from, to),
            Operation::Copy { from, to } => fs.copy(from, to),
            Operation::CreateDirectory(path) => fs.create_dir(path),
            Operation::CreateFile(path) => fs.create_file(path),
            Operation::Trash(path) => fs.trash(path),
        };
        if let Err(error) = result {
            return Outcome {
                applied,
                error: Some(format!("{}: {error}", describe(operation))),
            };
        }
        applied = applied.saturating_add(1);
    }
    Outcome {
        applied,
        error: None,
    }
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
