mod confirm;
mod delete;
mod path;

pub(super) use confirm::{ConfirmAction, ConfirmOutcome, ConfirmSession, PendingWrite, WriteKind};
pub(super) use delete::{DeleteOutcome, DeleteRequest, DeleteSession};
pub(super) use path::{PathAction, PathOutcome, PathPrompt};

use kawi_tui::frame::PromptMenu;

pub(crate) enum FileOpSession {
    Path(PathPrompt),
    Confirm(ConfirmSession),
    Delete(DeleteSession),
}

impl FileOpSession {
    pub(super) fn prompt(&self) -> PromptMenu<'_> {
        match self {
            Self::Path(prompt) => prompt.prompt(),
            Self::Confirm(prompt) => prompt.prompt(),
            Self::Delete(prompt) => prompt.prompt(),
        }
    }
}
