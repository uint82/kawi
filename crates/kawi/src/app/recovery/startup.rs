use std::path::PathBuf;

use super::App;

pub(crate) struct PendingRecoveryStart {
    root: PathBuf,
    cwd: PathBuf,
    restrict_to_open: bool,
}

impl App {
    pub(crate) fn schedule_startup_recovery(
        &mut self,
        root: PathBuf,
        cwd: PathBuf,
        restrict_to_open: bool,
    ) {
        self.pending_startup_recovery = Some(PendingRecoveryStart {
            root,
            cwd,
            restrict_to_open,
        });
    }

    pub(crate) fn start_pending_recovery(&mut self) -> bool {
        let Some(pending) = self.pending_startup_recovery.take() else {
            return false;
        };
        if let Some(warning) =
            self.start_recovery(&pending.root, &pending.cwd, pending.restrict_to_open)
        {
            self.show_startup_warning(warning);
        }
        true
    }
}

#[cfg(test)]
mod tests;
