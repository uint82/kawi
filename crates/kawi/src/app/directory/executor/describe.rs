use super::super::planner::Operation;

pub(super) fn describe(operation: &Operation) -> String {
    match operation {
        Operation::Rename { from, to, .. } => {
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

pub(in crate::app::directory) fn applied_description(operation: &Operation) -> String {
    match operation {
        Operation::Rename { from, to, .. } => {
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
