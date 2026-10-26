use super::super::*;
use super::convert::workspace_symbol_entry;
use super::document::document_symbols;
use super::workspace::workspace_symbols;
use super::{PendingSymbolKind, PendingSymbols};
use crate::protocol::{RequestId, ResponseError};
use lsp_types::WorkspaceSymbol;
use serde_json::Value;
use std::collections::HashMap;

impl Manager {
    pub(crate) fn handle_symbols_response(
        &mut self,
        key: ClientKey,
        id: RequestId,
        result: std::result::Result<Value, ResponseError>,
    ) {
        let Some(reply) = self.pending_replies.get(&id) else {
            return;
        };
        if reply.key != key {
            return;
        }
        let Some(reply) = self.pending_replies.remove(&id) else {
            return;
        };
        let Some(pending) = self.pending_symbols.get_mut(&reply.request) else {
            return;
        };
        match result {
            Ok(value) => pending.responses.push(super::SymbolResponse {
                server: reply.key.server.clone(),
                key: reply.key,
                value,
                encoding: reply.encoding,
            }),
            Err(error) => {
                if pending.error.is_none() {
                    pending.error = Some(one_line_message(&error.message));
                }
            }
        }
        self.advance_symbols(reply.request);
    }

    pub(crate) fn fail_symbol_request(&mut self, request: u64, message: String) {
        let Some(pending) = self.pending_symbols.get_mut(&request) else {
            return;
        };
        if pending.error.is_none() {
            pending.error = Some(message);
        }
        self.advance_symbols(request);
    }

    fn advance_symbols(&mut self, request: u64) {
        let Some(pending) = self.pending_symbols.get_mut(&request) else {
            return;
        };
        pending.remaining = pending.remaining.saturating_sub(1);
        if pending.remaining > 0 {
            return;
        }
        let Some(pending) = self.pending_symbols.remove(&request) else {
            return;
        };
        self.finish_symbols(pending);
    }

    fn finish_symbols(&mut self, pending: PendingSymbols) {
        match pending.kind.clone() {
            PendingSymbolKind::Document {
                path,
                version,
                text,
            } => self.finish_document_symbols(pending, path, version, text),
            PendingSymbolKind::Workspace { query } => self.finish_workspace_symbols(pending, query),
        }
    }

    fn finish_document_symbols(
        &mut self,
        pending: PendingSymbols,
        path: PathBuf,
        version: u64,
        text: Rope,
    ) {
        let mut symbols = Vec::new();
        for response in &pending.responses {
            match document_symbols(
                &self.documents,
                &path,
                &text,
                &response.value,
                &response.encoding,
            ) {
                Ok(mut entries) => symbols.append(&mut entries),
                Err(error) => {
                    self.emit(LspEvent::RequestError {
                        request: pending.request,
                        message: error,
                    });
                    return;
                }
            }
        }
        if symbols.is_empty() {
            self.emit(LspEvent::RequestError {
                request: pending.request,
                message: pending
                    .error
                    .unwrap_or_else(|| "No symbols found".to_string()),
            });
            return;
        }
        self.emit(LspEvent::DocumentSymbols(DocumentSymbolEvent {
            request: pending.request,
            path,
            version,
            symbols,
        }));
    }

    fn finish_workspace_symbols(&mut self, pending: PendingSymbols, query: String) {
        let mut symbols = Vec::new();
        let first_new = self.next_symbol_item;
        let mut responses = pending.responses;
        responses.sort_by(|left, right| left.server.cmp(&right.server));
        for response in responses {
            let resolve_support = self
                .clients
                .get(&response.key)
                .is_some_and(|client| client.support.workspace_symbol_resolve);
            match workspace_symbols(
                &mut self.symbol_resolve_items,
                &mut self.next_symbol_item,
                &self.documents,
                response,
                resolve_support,
                pending.resolve_support,
            ) {
                Ok(mut entries) => symbols.append(&mut entries),
                Err(error) => {
                    self.emit(LspEvent::RequestError {
                        request: pending.request,
                        message: error,
                    });
                    return;
                }
            }
        }
        if symbols.is_empty() {
            self.emit(LspEvent::RequestError {
                request: pending.request,
                message: pending
                    .error
                    .unwrap_or_else(|| "No workspace symbols found".to_string()),
            });
            return;
        }
        let active: Vec<u64> = self
            .pending_symbol_resolves
            .values()
            .map(|resolve| resolve.symbol_id)
            .collect();
        self.symbol_resolve_items
            .retain(|id, _| *id > first_new || active.contains(id));
        self.emit(LspEvent::WorkspaceSymbols(WorkspaceSymbolEvent {
            request: pending.request,
            query,
            symbols,
        }));
    }

    pub(crate) fn handle_symbol_resolve_response(
        &mut self,
        key: ClientKey,
        id: RequestId,
        result: std::result::Result<Value, ResponseError>,
    ) {
        let Some(reply) = self.pending_replies.get(&id) else {
            return;
        };
        if reply.key != key {
            return;
        }
        let Some(reply) = self.pending_replies.remove(&id) else {
            return;
        };
        let Some(pending) = self.pending_symbol_resolves.remove(&reply.request) else {
            return;
        };
        let resolved = match result {
            Ok(value) => serde_json::from_value::<WorkspaceSymbol>(value)
                .map_err(|error| format!("Invalid workspace symbol response: {error}")),
            Err(error) => Err(one_line_message(&error.message)),
        };
        let resolved = resolved.and_then(|symbol| {
            let Some(stored) = self.symbol_resolve_items.get(&pending.symbol_id) else {
                return Err("workspace symbol is no longer available".to_string());
            };
            let mut cache = HashMap::new();
            workspace_symbol_entry(
                &self.documents,
                &mut cache,
                &stored.path,
                &reply.encoding,
                symbol,
                &key.server,
                None,
            )
            .ok_or_else(|| "symbol has no file location".to_string())
        });
        match resolved {
            Ok(mut symbol) => {
                symbol.resolve_id = None;
                self.emit(LspEvent::WorkspaceSymbolResolved(
                    WorkspaceSymbolResolvedEvent {
                        request: reply.request,
                        symbol_id: pending.symbol_id,
                        symbol,
                    },
                ));
            }
            Err(error) => self.emit_text_error(reply.request, error),
        }
    }

    pub(crate) fn fail_symbol_resolve(&mut self, request: u64, message: String) {
        self.pending_symbol_resolves.remove(&request);
        self.emit_text_error(request, message);
    }
}
