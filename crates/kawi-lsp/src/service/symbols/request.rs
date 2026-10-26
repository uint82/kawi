use super::super::pending::ReplyPurpose;
use super::super::*;
use super::{PendingSymbolKind, PendingSymbolResolve, PendingSymbols};
use crate::workspace;
use lsp_types::{DocumentSymbolParams, TextDocumentIdentifier, WorkspaceSymbolParams};
use std::path::PathBuf;

impl Manager {
    pub(crate) fn start_document_symbols(&mut self, request: u64, path: PathBuf, version: u64) {
        let path = workspace::absolute(&path);
        let Some(document) = self.documents.get(&path) else {
            self.emit(LspEvent::RequestError {
                request,
                message: "document is not open".to_string(),
            });
            return;
        };
        let targets: Vec<(ClientKey, lsp_types::Uri)> = document
            .clients
            .iter()
            .filter_map(|key| {
                let client = self.clients.get(key)?;
                (client.status == ServerState::Ready
                    && client.opened.contains(&path)
                    && client.support.document_symbols)
                    .then(|| (key.clone(), document.uri.clone()))
            })
            .collect();
        if targets.is_empty() {
            self.emit(LspEvent::RequestError {
                request,
                message: "No language server supports document symbols".to_string(),
            });
            return;
        }
        let text = document.text.clone();
        self.pending_symbols.insert(
            request,
            PendingSymbols {
                request,
                kind: PendingSymbolKind::Document {
                    path,
                    version,
                    text,
                },
                remaining: targets.len(),
                responses: Vec::new(),
                resolve_support: false,
                error: None,
            },
        );
        for (key, uri) in targets {
            let params = DocumentSymbolParams {
                text_document: TextDocumentIdentifier { uri },
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
            };
            self.send_tracked_request(
                request,
                &key,
                "textDocument/documentSymbol",
                to_params(params),
                ReplyPurpose::Symbols,
            );
        }
    }

    pub(crate) fn start_workspace_symbols(&mut self, request: u64, query: String) {
        let targets: Vec<(ClientKey, bool)> = self
            .clients
            .iter()
            .filter(|(_, client)| {
                client.status == ServerState::Ready && client.support.workspace_symbols
            })
            .map(|(key, client)| (key.clone(), client.support.workspace_symbol_resolve))
            .collect();
        if targets.is_empty() {
            self.emit(LspEvent::RequestError {
                request,
                message: "No language server supports workspace symbols".to_string(),
            });
            return;
        }
        self.pending_symbols.insert(
            request,
            PendingSymbols {
                request,
                kind: PendingSymbolKind::Workspace {
                    query: query.clone(),
                },
                remaining: targets.len(),
                responses: Vec::new(),
                resolve_support: targets.iter().any(|(_, resolve)| *resolve),
                error: None,
            },
        );
        for (key, _) in targets {
            let params = WorkspaceSymbolParams {
                query: query.clone(),
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
            };
            self.send_tracked_request(
                request,
                &key,
                "workspace/symbol",
                to_params(params),
                ReplyPurpose::Symbols,
            );
        }
    }

    pub(crate) fn start_workspace_symbol_resolve(&mut self, request: u64, symbol_id: u64) {
        let Some(stored) = self.symbol_resolve_items.get(&symbol_id).cloned() else {
            self.emit_text_error(
                request,
                "workspace symbol is no longer available".to_string(),
            );
            return;
        };
        if !self.clients.contains_key(&stored.key) {
            self.emit_text_error(request, "language server is not available".to_string());
            return;
        }
        self.pending_symbol_resolves
            .insert(request, PendingSymbolResolve { symbol_id });
        self.send_tracked_request(
            request,
            &stored.key,
            "workspaceSymbol/resolve",
            to_params(stored.symbol),
            ReplyPurpose::SymbolResolve,
        );
    }

    pub(crate) fn cancel_symbol_request(&mut self, request: u64) -> bool {
        self.pending_symbols.remove(&request).is_some()
            || self.pending_symbol_resolves.remove(&request).is_some()
    }
}
