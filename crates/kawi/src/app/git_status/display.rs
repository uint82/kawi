use kawi_git::{BranchStatus, Status};
use kawi_tui::statusbar::GitStatusInfo;

use super::path::status_matches_path;
use super::GitStatusState;

const SHORT_OID: usize = 7;

impl GitStatusState {
    pub(in crate::app) fn display(&self) -> Option<GitStatusInfo<'_>> {
        let status = self.status.as_ref()?;
        if !status_matches_path(self.path.as_deref(), status) {
            return None;
        }
        let upstream = status.upstream();
        Some(GitStatusInfo {
            branch: branch_label(status),
            operation: status.operation.label(),
            ahead: upstream.map_or(0, |upstream| upstream.ahead),
            behind: upstream.map_or(0, |upstream| upstream.behind),
        })
    }
}

fn branch_label(status: &Status) -> &str {
    match &status.branch {
        BranchStatus::Named { name, .. } => {
            if name.is_empty() {
                "HEAD"
            } else {
                name
            }
        }
        BranchStatus::Detached { oid } => &oid[..oid.len().min(SHORT_OID)],
    }
}
