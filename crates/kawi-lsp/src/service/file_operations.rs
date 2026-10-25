use super::workspace_edit::EMPTY_WORKSPACE_EDIT;
use super::*;
use crate::protocol::Message;
use crate::workspace::{self, file_uri};
use lsp_types::{DeleteFilesParams, FileDelete, FileRename, RenameFilesParams};
use serde_json::Value;
use std::time::Duration;

mod will;

impl Manager {
    pub(super) fn did_rename_files(&mut self, files: &[(PathBuf, PathBuf)]) {
        let files: Vec<(PathBuf, PathBuf)> = normalize(files.to_vec())
            .into_iter()
            .filter(|(old, new)| old != new)
            .collect();
        if files.is_empty() {
            return;
        }
        if let Some(params) = file_operation_params(FileOperationKind::Rename, &files) {
            self.notify_file_operation("workspace/didRenameFiles", params, |support| {
                support.did_rename_files
            });
        }
        self.rekey_documents(&files);
    }

    pub(super) fn did_delete_files(&mut self, files: &[PathBuf]) {
        let files: Vec<PathBuf> = files.iter().map(|path| workspace::absolute(path)).collect();
        if files.is_empty() {
            return;
        }
        if let Some(params) = delete_params(&files) {
            self.notify_file_operation("workspace/didDeleteFiles", params, |support| {
                support.did_delete_files
            });
        }
        let removed: Vec<(PathBuf, Document)> = std::mem::take(&mut self.documents)
            .into_iter()
            .filter(|(path, _)| files.iter().any(|target| is_under(path, target)))
            .collect();
        for (path, document) in removed {
            for key in &document.clients {
                let Some(client) = self.clients.get_mut(key) else {
                    continue;
                };
                client.opened.remove(&path);
                if !client.support.did_delete_files && client.sync.open_close {
                    send_close(client, &document.uri);
                }
            }
            self.forget_pull_diagnostics(&path);
        }
    }

    fn notify_file_operation(
        &mut self,
        method: &str,
        params: Value,
        supports: impl Fn(&RequestSupport) -> bool,
    ) {
        for (_, client) in self
            .clients
            .iter()
            .filter(|(_, client)| client.status == ServerState::Ready && supports(&client.support))
        {
            client
                .client
                .send(Message::notification(method, Some(params.clone())));
        }
    }

    fn rekey_documents(&mut self, files: &[(PathBuf, PathBuf)]) {
        let documents = std::mem::take(&mut self.documents);
        for (path, document) in documents {
            let Some(new_path) = rewrite_any(&path, files) else {
                self.documents.insert(path, document);
                continue;
            };
            if self.document_attachment_changed(&document, &new_path) {
                self.reopen_document(path, document, new_path);
                continue;
            }
            let Ok(new_uri) = file_uri(&new_path) else {
                self.documents.insert(path, document);
                continue;
            };
            let text = document.text.clone();
            let version = document.version;
            let clients = document.clients.clone();
            let old_uri = document.uri.clone();
            self.documents.insert(
                new_path.clone(),
                Document {
                    uri: new_uri,
                    language_id: document.language_id,
                    text: document.text,
                    version: document.version,
                    clients: document.clients,
                },
            );
            for key in clients {
                let Some(client) = self.clients.get_mut(&key) else {
                    continue;
                };
                let was_open = client.opened.remove(&path);
                if !was_open {
                    continue;
                }
                if client.support.did_rename_files {
                    client.opened.insert(new_path.clone());
                } else {
                    if client.sync.open_close {
                        send_close(client, &old_uri);
                    }
                    self.send_open(&key, &new_path, &text, version);
                }
            }
            self.forget_pull_diagnostics(&path);
            self.schedule_pull_diagnostics(&new_path, Duration::ZERO);
        }
    }

    fn document_attachment_changed(&self, document: &Document, new_path: &Path) -> bool {
        let Some(language) = self.languages.language_for_path(new_path) else {
            return true;
        };
        let root = workspace::root_for(new_path, &language.root_markers);
        document.language_id != language.language_id
            || document
                .clients
                .iter()
                .any(|key| !language.servers.contains(&key.server) || key.root != root)
            || language
                .servers
                .iter()
                .any(|server| !document.clients.iter().any(|key| key.server == *server))
    }

    fn reopen_document(&mut self, path: PathBuf, document: Document, new_path: PathBuf) {
        for key in &document.clients {
            let Some(client) = self.clients.get_mut(key) else {
                continue;
            };
            if client.opened.remove(&path) && client.sync.open_close {
                send_close(client, &document.uri);
            }
        }
        self.forget_pull_diagnostics(&path);
        self.open_document(new_path, document.text);
    }

    fn parse_optional_workspace_edit(
        &self,
        value: Value,
        encoding: &PositionEncodingKind,
    ) -> Result<Option<WorkspaceEdit>, String> {
        if value.is_null() {
            return Ok(None);
        }
        match self.parse_workspace_edit(value, encoding) {
            Ok(edit) => Ok(Some(edit)),
            Err(message) if message == EMPTY_WORKSPACE_EDIT => Ok(None),
            Err(message) => Err(message),
        }
    }
}

fn send_close(client: &mut ClientState, uri: &lsp_types::Uri) {
    let params = lsp_types::DidCloseTextDocumentParams {
        text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
    };
    if let Some(params) = to_params(params) {
        client
            .client
            .send(Message::notification("textDocument/didClose", Some(params)));
    }
}

fn merge_edit(target: &mut Option<WorkspaceEdit>, edit: WorkspaceEdit) {
    match target {
        Some(existing) => {
            for document in edit.documents {
                if !existing
                    .documents
                    .iter()
                    .any(|current| current.path == document.path)
                {
                    existing.documents.push(document);
                }
            }
        }
        None => *target = Some(edit),
    }
}

pub(super) fn file_operation_params(
    kind: FileOperationKind,
    files: &[(PathBuf, PathBuf)],
) -> Option<Value> {
    match kind {
        FileOperationKind::Rename => {
            let files = files
                .iter()
                .map(|(old, new)| {
                    Some(FileRename {
                        old_uri: file_uri(old).ok()?.to_string(),
                        new_uri: file_uri(new).ok()?.to_string(),
                    })
                })
                .collect::<Option<Vec<_>>>()?;
            if files.is_empty() {
                None
            } else {
                to_params(RenameFilesParams { files })
            }
        }
        FileOperationKind::Delete => {
            let files = files
                .iter()
                .map(|(path, _)| file_uri(path).ok())
                .collect::<Option<Vec<_>>>()?;
            delete_params_from_uris(&files)
        }
    }
}

pub(super) fn delete_params(files: &[PathBuf]) -> Option<Value> {
    let uris = files
        .iter()
        .map(|path| file_uri(path).ok())
        .collect::<Option<Vec<_>>>()?;
    delete_params_from_uris(&uris)
}

fn delete_params_from_uris(uris: &[lsp_types::Uri]) -> Option<Value> {
    let files = uris
        .iter()
        .map(|uri| FileDelete {
            uri: uri.to_string(),
        })
        .collect::<Vec<_>>();
    if files.is_empty() {
        None
    } else {
        to_params(DeleteFilesParams { files })
    }
}

pub(super) fn normalize(files: Vec<(PathBuf, PathBuf)>) -> Vec<(PathBuf, PathBuf)> {
    files
        .into_iter()
        .map(|(old, new)| (workspace::absolute(&old), workspace::absolute(&new)))
        .collect()
}

fn rewrite_any(path: &Path, files: &[(PathBuf, PathBuf)]) -> Option<PathBuf> {
    files.iter().find_map(|(old, new)| rewrite(path, old, new))
}

fn rewrite(path: &Path, old: &Path, new: &Path) -> Option<PathBuf> {
    if path == old {
        return Some(new.to_path_buf());
    }
    let rest = path.strip_prefix(old).ok()?;
    (!rest.as_os_str().is_empty()).then(|| new.join(rest))
}

fn is_under(path: &Path, root: &Path) -> bool {
    path == root
        || path
            .strip_prefix(root)
            .is_ok_and(|rest| !rest.as_os_str().is_empty())
}
