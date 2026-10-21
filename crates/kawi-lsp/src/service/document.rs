use super::initialize::{include_save_text, save_enabled};
use super::*;
use crate::protocol::Message;
use crate::workspace::{self, file_uri, whole_document_range};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, TextDocumentContentChangeEvent, TextDocumentItem,
    TextDocumentSyncKind, VersionedTextDocumentIdentifier,
};
use std::path::{Path, PathBuf};

impl Manager {
    pub(super) fn open_document(&mut self, path: PathBuf, text: String) {
        let path = workspace::absolute(&path);
        let Some(language) = self.languages.language_for_path(&path).cloned() else {
            return;
        };
        self.close_document(&path);
        let root = workspace::root_for(&path, &language.root_markers);
        let Ok(uri) = file_uri(&path) else {
            return;
        };
        let mut keys = Vec::new();
        for server in &language.servers {
            let Some(config) = self.languages.server(server).cloned() else {
                continue;
            };
            if !config.enabled {
                continue;
            }
            if !self.languages.server_available(server) {
                self.emit_status(server, ServerState::Missing, None);
                continue;
            }
            let key = ClientKey {
                server: server.clone(),
                root: root.clone(),
            };
            if !self.clients.contains_key(&key) {
                self.start_client(key.clone(), &config, &root);
            }
            if self.clients.contains_key(&key) {
                keys.push(key);
            }
        }
        let document = Document {
            uri,
            language_id: language.language_id,
            text,
            version: INITIAL_DOCUMENT_VERSION,
            clients: keys.clone(),
        };
        self.documents.insert(path.clone(), document);
        for key in keys {
            if self
                .clients
                .get(&key)
                .is_some_and(|client| client.status == ServerState::Ready)
            {
                let (text, version) = self
                    .documents
                    .get(&path)
                    .map(|document| (document.text.clone(), document.version))
                    .unwrap_or_default();
                self.send_open(&key, &path, &text, version);
            }
        }
    }

    pub(super) fn change_document(&mut self, path: &Path, text: String) {
        let path = workspace::absolute(path);
        let Some(document) = self.documents.get_mut(&path) else {
            return;
        };
        document.text = text;
        document.version = document.version.saturating_add(1);
        let version = document.version;
        let uri = document.uri.clone();
        let text = document.text.clone();
        let clients = document.clients.clone();
        for key in clients {
            self.send_change(&key, &path, &uri, version, &text);
        }
    }

    pub(super) fn send_change(
        &mut self,
        key: &ClientKey,
        path: &Path,
        uri: &lsp_types::Uri,
        version: i32,
        text: &str,
    ) {
        let Some(client) = self.clients.get(key) else {
            return;
        };
        if client.status != ServerState::Ready
            || !client.opened.contains(path)
            || client.sync.change.is_none()
        {
            return;
        }
        let change = match client.sync.change {
            Some(TextDocumentSyncKind::FULL) => TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: text.to_string(),
            },
            Some(TextDocumentSyncKind::INCREMENTAL) => TextDocumentContentChangeEvent {
                range: Some(whole_document_range(text, &client.encoding)),
                range_length: None,
                text: text.to_string(),
            },
            _ => return,
        };
        let params = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version,
            },
            content_changes: vec![change],
        };
        if let Some(params) = to_params(params) {
            client.client.send(Message::notification(
                "textDocument/didChange",
                Some(params),
            ));
        }
    }

    pub(super) fn save_document(&mut self, path: &Path) {
        let path = workspace::absolute(path);
        let Some(document) = self.documents.get(&path) else {
            return;
        };
        let mut messages = Vec::new();
        for key in &document.clients {
            let Some(client) = self.clients.get(key) else {
                continue;
            };
            if !client.opened.contains(&path) {
                continue;
            }
            let Some(save) = &client.sync.save else {
                continue;
            };
            if !save_enabled(save) {
                continue;
            }
            let text = include_save_text(save).then(|| document.text.clone());
            let params = DidSaveTextDocumentParams {
                text_document: lsp_types::TextDocumentIdentifier {
                    uri: document.uri.clone(),
                },
                text,
            };
            if let Some(params) = to_params(params) {
                messages.push((
                    key.clone(),
                    Message::notification("textDocument/didSave", Some(params)),
                ));
            }
        }
        for (key, message) in messages {
            if let Some(client) = self.clients.get(&key) {
                client.client.send(message);
            }
        }
    }

    pub(super) fn close_document(&mut self, path: &Path) {
        let path = workspace::absolute(path);
        let Some(document) = self.documents.remove(&path) else {
            return;
        };
        for key in document.clients {
            if let Some(client) = self.clients.get_mut(&key) {
                if client.opened.remove(&path) && client.sync.open_close {
                    let params = DidCloseTextDocumentParams {
                        text_document: lsp_types::TextDocumentIdentifier {
                            uri: document.uri.clone(),
                        },
                    };
                    if let Some(params) = to_params(params) {
                        client
                            .client
                            .send(Message::notification("textDocument/didClose", Some(params)));
                    }
                }
            }
        }
    }

    pub(super) fn send_open(&mut self, key: &ClientKey, path: &Path, text: &str, version: i32) {
        let Some(client) = self.clients.get_mut(key) else {
            return;
        };
        if client.status != ServerState::Ready || !client.sync.open_close {
            return;
        }
        let Some(document) = self.documents.get(path) else {
            return;
        };
        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: document.uri.clone(),
                language_id: document.language_id.clone(),
                version,
                text: text.to_string(),
            },
        };
        if let Some(params) = to_params(params) {
            if client
                .client
                .send(Message::notification("textDocument/didOpen", Some(params)))
            {
                client.opened.insert(path.to_path_buf());
            }
        }
    }
}
