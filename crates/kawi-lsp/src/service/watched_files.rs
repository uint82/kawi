use super::*;
use crate::protocol::Message;
use crate::workspace::file_uri;
use lsp_types::{DidChangeWatchedFilesParams, FileChangeType, FileEvent};

impl Manager {
    pub(super) fn did_change_watched_files(&mut self, files: &[(PathBuf, WatchedFileChange)]) {
        let changes: Vec<FileEvent> = files
            .iter()
            .filter_map(|(path, change)| {
                Some(FileEvent {
                    uri: file_uri(path).ok()?,
                    typ: file_change_type(*change),
                })
            })
            .collect();
        if changes.is_empty() {
            return;
        }
        let params = DidChangeWatchedFilesParams { changes };
        let Some(params) = to_params(params) else {
            return;
        };
        let message = Message::notification("workspace/didChangeWatchedFiles", Some(params));
        for client in self
            .clients
            .values()
            .filter(|client| client.status == ServerState::Ready)
        {
            client.client.send(message.clone());
        }
    }
}

fn file_change_type(change: WatchedFileChange) -> FileChangeType {
    match change {
        WatchedFileChange::Created => FileChangeType::CREATED,
        WatchedFileChange::Changed => FileChangeType::CHANGED,
        WatchedFileChange::Deleted => FileChangeType::DELETED,
    }
}
