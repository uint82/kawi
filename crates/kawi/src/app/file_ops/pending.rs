use std::path::PathBuf;

use super::super::directory::PendingApply;
use super::prompt::DeleteRequest;
use super::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(crate) enum PendingFileOp {
    Rename(RenamePending),
    Delete(DeletePending),
    Directory(PendingDirectoryOp),
}

pub(crate) struct PendingDirectoryOp {
    pub(crate) request: u64,
    pub(crate) apply: PendingApply,
}

pub(crate) struct RenamePending {
    pub(crate) request: u64,
    pub(crate) source: PathBuf,
    pub(crate) target: PathBuf,
    pub(crate) force: bool,
    pub(crate) directory: bool,
}

pub(crate) struct DeletePending {
    pub(crate) request: u64,
    pub(crate) delete: DeleteRequest,
    pub(crate) keep_dirty: bool,
}

impl PendingFileOp {
    fn request(&self) -> u64 {
        match self {
            Self::Rename(pending) => pending.request,
            Self::Delete(pending) => pending.request,
            Self::Directory(pending) => pending.request,
        }
    }

    fn finish(self, app: &mut App) {
        match self {
            Self::Rename(pending) => app.finish_rename(
                pending.source,
                pending.target,
                pending.force,
                pending.directory,
            ),
            Self::Delete(pending) => app.finish_delete(pending.delete, pending.keep_dirty),
            Self::Directory(pending) => {
                app.apply_directory_changes(pending.apply);
            }
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Rename(_) => "Rename",
            Self::Delete(_) => "Delete",
            Self::Directory(_) => "Directory",
        }
    }

    fn cancel(self, app: &mut App) {
        if let Self::Directory(pending) = self {
            app.cancel_directory_apply(&pending.apply.edits);
        }
    }
}

impl App {
    pub(crate) fn handle_file_operation_result(&mut self) {
        let Some(event) = self.lsp.take_file_operation() else {
            return;
        };
        let Some(pending) = self.pending_file_op.take() else {
            return;
        };
        if pending.request() != event.request {
            self.pending_file_op = Some(pending);
            return;
        }
        if let Some(error) = event.error {
            self.show_message(format!("{} cancelled: {error}", pending.label()));
            pending.cancel(self);
            return;
        }
        if let Some(edit) = event.edit {
            if let Err(message) = self.apply_workspace_edit(&edit) {
                self.show_message(format!("{} cancelled: {message}", pending.label()));
                pending.cancel(self);
                return;
            }
        }
        pending.finish(self);
    }

    pub(crate) fn handle_pending_file_op_key(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        if is_cancel_key(key) {
            if let Some(pending) = self.pending_file_op.take() {
                self.show_message(format!("{} cancelled", pending.label()));
                pending.cancel(self);
            }
        }
        Ok(false)
    }
}

fn is_cancel_key(key: KeyEvent) -> bool {
    key.code == KeyCode::Esc
        || (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c'))
}
