use kawi_git::{Branch, BranchList, CommitFile, LogOptions};

pub(crate) struct PendingSwitch {
    pub(super) branch: Branch,
}

pub(crate) struct PendingDelete {
    pub(super) branch: Branch,
}

#[derive(Default)]
pub(crate) struct GitBranchState {
    pub(super) branches: Option<BranchList>,
    pub(super) commit_sha: Option<String>,
    pub(super) commit_files: Vec<CommitFile>,
    pub(super) log_options: Option<LogOptions>,
    pub(super) pending_switch: Option<PendingSwitch>,
    pub(super) pending_delete: Option<PendingDelete>,
    pub(super) pending_checkout: Option<String>,
}
