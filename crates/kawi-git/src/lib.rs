mod branch;
mod command;
mod hunks;
mod load_service;
mod log;
mod ops;
mod patch;
mod repo;
mod service;
mod status;
mod version;

pub use branch::{
    create, delete, is_unborn, list as list_branches, switch, switch_detached, switch_ref, Branch,
    BranchKind, BranchList, BranchListOptions, CreateOutcome, DeleteOutcome, SwitchMode,
    SwitchOutcome,
};
pub use command::resolve_root as repository_root;
pub use hunks::{
    apply_hunks, diff_hunks, filter_common, hunk_at, next_hunk, prev_hunk, Hunk, HunkKind, Sign,
    SignKind,
};
pub use load_service::{FileLoadEvent, FileLoadService};
pub use log::{
    commit_files, commit_patch, file_diff, CommitFile, LogEntry, LogEvent, LogOptions, LogStream,
};
pub use ops::{clean_path, diff_path, git_dir, restore_path, stage_path, unstage_path};
pub use patch::{patch_hunks, PatchSettings};
pub use repo::{apply_index_patch, ensure_intent_to_add, load, reset_index, GitFile};
pub use service::{StatusEvent, StatusService};
pub use status::{
    parse_status, BranchStatus, EntryKind, OperationState, Status, StatusCode, StatusEntry,
    Upstream,
};
pub use version::{git_version, GitVersion};
