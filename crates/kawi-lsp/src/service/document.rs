use super::initialize::{include_save_text, save_enabled};
use super::*;
use crate::protocol::Message;
use crate::text;
use crate::workspace::{self, file_uri};
use kawi_buffer::{diff_ropes, RopeChange};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, Range, TextDocumentContentChangeEvent, TextDocumentItem,
    TextDocumentSyncKind, VersionedTextDocumentIdentifier,
};
use ropey::Rope;
use std::path::{Path, PathBuf};

struct DocumentChange<'a> {
    path: &'a Path,
    uri: &'a lsp_types::Uri,
    version: i32,
    old: &'a Rope,
    new: &'a Rope,
    change: Option<RopeChange>,
}

impl Manager {
    pub(super) fn open_document(&mut self, path: PathBuf, text: Rope) {
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

    pub(super) fn change_document(&mut self, path: &Path, text: Rope) {
        let path = workspace::absolute(path);
        let Some(document) = self.documents.get_mut(&path) else {
            return;
        };
        let old = std::mem::replace(&mut document.text, text);
        let change = diff_ropes(&old, &document.text);
        document.version = document.version.saturating_add(1);
        let version = document.version;
        let uri = document.uri.clone();
        let new = document.text.clone();
        let clients = document.clients.clone();
        let change = DocumentChange {
            path: &path,
            uri: &uri,
            version,
            old: &old,
            new: &new,
            change,
        };
        for key in clients {
            self.send_change(&key, &change);
        }
    }

    fn send_change(&mut self, key: &ClientKey, change: &DocumentChange<'_>) {
        let Some(client) = self.clients.get(key) else {
            return;
        };
        if client.status != ServerState::Ready
            || !client.opened.contains(change.path)
            || client.sync.change.is_none()
        {
            return;
        }
        let changes = match client.sync.change {
            Some(TextDocumentSyncKind::FULL) => vec![full_change(change.new)],
            Some(TextDocumentSyncKind::INCREMENTAL) => {
                incremental_changes(change.change, change.old, change.new, &client.encoding)
            }
            _ => return,
        };
        let params = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: change.uri.clone(),
                version: change.version,
            },
            content_changes: changes,
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
            let text = include_save_text(save).then(|| document.text.to_string());
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

    pub(super) fn send_open(&mut self, key: &ClientKey, path: &Path, text: &Rope, version: i32) {
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

fn full_change(text: &Rope) -> TextDocumentContentChangeEvent {
    TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        text: text.to_string(),
    }
}

fn incremental_changes(
    change: Option<RopeChange>,
    old: &Rope,
    new: &Rope,
    encoding: &lsp_types::PositionEncodingKind,
) -> Vec<TextDocumentContentChangeEvent> {
    let change = change.unwrap_or(RopeChange {
        start: 0,
        old_end: 0,
        new_end: 0,
    });
    let range = Range::new(
        text::position(old, change.start, encoding),
        text::position(old, change.old_end, encoding),
    );
    let text = new.slice(change.start..change.new_end).to_string();
    vec![TextDocumentContentChangeEvent {
        range: Some(range),
        range_length: None,
        text,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::{Position, PositionEncodingKind};

    fn incremental(
        old: &str,
        new: &str,
        encoding: &PositionEncodingKind,
    ) -> TextDocumentContentChangeEvent {
        let old = Rope::from_str(old);
        let new = Rope::from_str(new);
        let change = diff_ropes(&old, &new);
        incremental_changes(change, &old, &new, encoding)
            .into_iter()
            .next()
            .expect("change")
    }

    #[test]
    fn reports_utf16_positions_for_insertions() {
        let change = incremental("a\n", "a\n𝄞", &PositionEncodingKind::UTF16);
        assert_eq!(
            change.range,
            Some(Range::new(Position::new(1, 0), Position::new(1, 0)))
        );
        assert_eq!(change.text, "𝄞");
    }

    #[test]
    fn reports_utf8_and_utf32_positions_for_replacements() {
        let utf8 = incremental("a𝄞b", "a𝄢b", &PositionEncodingKind::UTF8);
        assert_eq!(
            utf8.range,
            Some(Range::new(Position::new(0, 1), Position::new(0, 5)))
        );
        assert_eq!(utf8.text, "𝄢");
        let utf32 = incremental("a𝄞b", "a𝄢b", &PositionEncodingKind::UTF32);
        assert_eq!(
            utf32.range,
            Some(Range::new(Position::new(0, 1), Position::new(0, 2)))
        );
    }

    #[test]
    fn unchanged_documents_produce_a_noop_change() {
        let change = incremental("same", "same", &PositionEncodingKind::UTF16);
        assert_eq!(
            change.range,
            Some(Range::new(Position::new(0, 0), Position::new(0, 0)))
        );
        assert!(change.text.is_empty());
    }
}
