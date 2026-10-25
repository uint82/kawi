mod list;
mod manage;
mod ops;
mod parser;
mod worktree;

pub use list::{is_unborn, list, Branch, BranchKind, BranchList, BranchListOptions};
pub use manage::{create, delete, CreateOutcome, DeleteOutcome};
pub use ops::{switch, switch_detached, switch_ref, SwitchMode, SwitchOutcome};

#[cfg(test)]
mod tests;
