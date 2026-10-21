mod hunks;
mod patch;
mod repo;

pub use hunks::{
    apply_hunks, diff_hunks, filter_common, hunk_at, next_hunk, prev_hunk, Hunk, HunkKind, Sign,
    SignKind,
};
pub use patch::{patch_hunks, PatchSettings};
pub use repo::{apply_index_patch, ensure_intent_to_add, load, reset_index, GitFile};
