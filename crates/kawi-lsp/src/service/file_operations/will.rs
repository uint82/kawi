use super::pending::{PendingReply, ReplyPurpose};
use super::*;
use crate::protocol::Message;
use std::time::{Duration, Instant};

const FILE_OPERATION_TIMEOUT: Duration = Duration::from_secs(10);

impl Manager {
    pub(crate) fn start_will_rename_files(&mut self, request: u64, files: Vec<(PathBuf, PathBuf)>) {
        let files = normalize(files)
            .into_iter()
            .filter(|(old, new)| old != new)
            .collect();
        self.start_will_file_operation(request, FileOperationKind::Rename, files);
    }

    pub(crate) fn start_will_delete_files(&mut self, request: u64, files: Vec<PathBuf>) {
        let files = normalize(files.into_iter().map(|path| (path.clone(), path)).collect());
        self.start_will_file_operation(request, FileOperationKind::Delete, files);
    }

    fn start_will_file_operation(
        &mut self,
        request: u64,
        kind: FileOperationKind,
        files: Vec<(PathBuf, PathBuf)>,
    ) {
        let method = match kind {
            FileOperationKind::Rename => "workspace/willRenameFiles",
            FileOperationKind::Delete => "workspace/willDeleteFiles",
        };
        let params = file_operation_params(kind, &files);
        let targets: Vec<ClientKey> = self
            .clients
            .iter()
            .filter(|(_, client)| {
                client.status == ServerState::Ready
                    && match kind {
                        FileOperationKind::Rename => client.support.will_rename_files,
                        FileOperationKind::Delete => client.support.will_delete_files,
                    }
            })
            .map(|(key, _)| key.clone())
            .collect();
        if targets.is_empty() {
            self.emit(LspEvent::FileOperation(FileOperationEvent {
                request,
                kind,
                edit: None,
                error: None,
            }));
            return;
        }
        let Some(params) = params else {
            self.emit(LspEvent::FileOperation(FileOperationEvent {
                request,
                kind,
                edit: None,
                error: Some("file operation params could not be serialized".to_string()),
            }));
            return;
        };
        let mut remaining = HashSet::new();
        for key in targets {
            let id = self.request_id();
            let encoding = self
                .clients
                .get(&key)
                .map_or(PositionEncodingKind::UTF16, |client| {
                    client.encoding.clone()
                });
            self.pending_replies.insert(
                id.clone(),
                PendingReply {
                    request,
                    key: key.clone(),
                    encoding,
                    purpose: ReplyPurpose::FileOperation,
                },
            );
            let sent = self.clients.get(&key).is_some_and(|client| {
                client
                    .client
                    .send(Message::request(id.clone(), method, Some(params.clone())))
            });
            if sent {
                remaining.insert(key);
            } else {
                self.pending_replies.remove(&id);
            }
        }
        if remaining.is_empty() {
            self.emit(LspEvent::FileOperation(FileOperationEvent {
                request,
                kind,
                edit: None,
                error: Some("servers are not accepting messages".to_string()),
            }));
            return;
        }
        self.pending_file_operations.insert(
            request,
            PendingFileOperation {
                kind,
                remaining,
                edit: None,
                deadline: Instant::now() + FILE_OPERATION_TIMEOUT,
            },
        );
    }

    pub(crate) fn handle_file_operation_response(
        &mut self,
        key: ClientKey,
        request: u64,
        result: std::result::Result<Value, crate::protocol::ResponseError>,
    ) {
        let Some(mut pending) = self.pending_file_operations.remove(&request) else {
            return;
        };
        pending.remaining.remove(&key);
        let encoding = self
            .clients
            .get(&key)
            .map_or(PositionEncodingKind::UTF16, |client| {
                client.encoding.clone()
            });
        match result {
            Ok(value) => match self.parse_optional_workspace_edit(value, &encoding) {
                Ok(Some(edit)) => merge_edit(&mut pending.edit, edit),
                Ok(None) => {}
                Err(message) => {
                    self.fail_file_operation(request, message);
                    return;
                }
            },
            Err(error) => {
                self.fail_file_operation(request, error.message);
                return;
            }
        }
        if pending.remaining.is_empty() {
            self.emit(LspEvent::FileOperation(FileOperationEvent {
                request,
                kind: pending.kind,
                edit: pending.edit,
                error: None,
            }));
        } else {
            self.pending_file_operations.insert(request, pending);
        }
    }

    pub(crate) fn fail_file_operation(&mut self, request: u64, message: String) {
        let Some(pending) = self.pending_file_operations.remove(&request) else {
            return;
        };
        self.pending_replies
            .retain(|_, reply| reply.request != request);
        self.emit(LspEvent::FileOperation(FileOperationEvent {
            request,
            kind: pending.kind,
            edit: None,
            error: Some(message),
        }));
    }

    pub(crate) fn expire_file_operations(&mut self) {
        let expired: Vec<u64> = self
            .pending_file_operations
            .iter()
            .filter(|(_, pending)| pending.deadline <= Instant::now())
            .map(|(request, _)| *request)
            .collect();
        for request in expired {
            self.fail_file_operation(request, "file operation timed out".to_string());
        }
    }

    pub(crate) fn next_file_operation_deadline(&self) -> Option<Duration> {
        self.pending_file_operations
            .values()
            .map(|pending| pending.deadline.saturating_duration_since(Instant::now()))
            .min()
    }
}
