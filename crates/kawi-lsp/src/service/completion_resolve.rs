use super::completion_edit::convert_edit;
use super::markup::markup_document;
use super::pending::{PendingReply, ReplyPurpose};
use super::*;
use crate::client::ClientKey;
use crate::protocol::{RequestId, ResponseError};
use lsp_types::CompletionItem;
use serde_json::Value;

#[derive(Debug, Clone)]
pub(super) struct ResolveItem {
    pub(super) key: ClientKey,
    pub(super) item: CompletionItem,
    pub(super) path: PathBuf,
    pub(super) version: i32,
    pub(super) original_hash: u64,
}

#[derive(Debug, Clone)]
pub(super) struct PendingResolve {
    pub(super) item_id: u64,
    pub(super) path: PathBuf,
    pub(super) version: i32,
    pub(super) original_hash: u64,
}

impl Manager {
    pub(super) fn store_resolve_item(
        &mut self,
        id: u64,
        key: ClientKey,
        item: CompletionItem,
        path: PathBuf,
        version: i32,
        original_hash: u64,
    ) {
        self.completion_resolve.insert(
            id,
            ResolveItem {
                key,
                item,
                path,
                version,
                original_hash,
            },
        );
    }

    pub(super) fn clear_resolve_items(&mut self) {
        self.completion_resolve.clear();
    }

    pub(super) fn start_completion_resolve(&mut self, request: u64, item_id: u64) {
        let Some(resolve) = self.completion_resolve.get(&item_id).cloned() else {
            self.emit_text_error(
                request,
                "completion item is no longer available".to_string(),
            );
            return;
        };
        if !self.clients.contains_key(&resolve.key) {
            self.emit_text_error(request, "language server is not available".to_string());
            return;
        }
        self.pending_completion_resolves.insert(
            request,
            PendingResolve {
                item_id,
                path: resolve.path.clone(),
                version: resolve.version,
                original_hash: resolve.original_hash,
            },
        );
        self.send_tracked_request(
            request,
            &resolve.key,
            "completionItem/resolve",
            to_params(resolve.item),
            ReplyPurpose::CompletionResolve,
        );
    }

    pub(super) fn handle_completion_resolve_response(
        &mut self,
        key: ClientKey,
        id: RequestId,
        result: std::result::Result<Value, ResponseError>,
    ) {
        let Some((reply, pending)) = self.take_completion_resolve(key, id) else {
            return;
        };
        let item = match completion_item(result) {
            Ok(item) => item,
            Err(error) => {
                self.emit_text_error(reply.request, error);
                return;
            }
        };
        let Some(document) = self.documents.get(&pending.path) else {
            self.emit_text_error(reply.request, "document is not open".to_string());
            return;
        };
        if document.version != pending.version {
            self.emit_text_error(reply.request, "document changed".to_string());
            return;
        }
        let additional = match additional_edits(
            &document.text,
            item.additional_text_edits.as_deref(),
            &reply.encoding,
        ) {
            Ok(additional) => additional,
            Err(error) => {
                self.emit_text_error(reply.request, error);
                return;
            }
        };
        self.emit(LspEvent::CompletionResolved(CompletionResolveEvent {
            request: reply.request,
            item_id: pending.item_id,
            original_hash: pending.original_hash,
            detail: item.detail.as_deref().map(one_line_message),
            documentation: item.documentation.map(markup_document),
            additional,
        }));
    }

    fn take_completion_resolve(
        &mut self,
        key: ClientKey,
        id: RequestId,
    ) -> Option<(PendingReply, PendingResolve)> {
        let reply = self.pending_replies.get(&id)?;
        if reply.key != key {
            return None;
        }
        let reply = self.pending_replies.remove(&id)?;
        let pending = self.pending_completion_resolves.remove(&reply.request)?;
        Some((reply, pending))
    }
}

fn completion_item(
    result: std::result::Result<Value, ResponseError>,
) -> Result<CompletionItem, String> {
    match result {
        Ok(value) => serde_json::from_value(value)
            .map_err(|error| format!("Invalid completion resolve response: {error}")),
        Err(error) => Err(one_line_message(&error.message)),
    }
}

fn additional_edits(
    text: &Rope,
    edits: Option<&[lsp_types::TextEdit]>,
    encoding: &PositionEncodingKind,
) -> Result<Vec<TextChange>, String> {
    edits
        .unwrap_or_default()
        .iter()
        .map(|edit| {
            convert_edit(text, &edit.range, &edit.new_text, encoding)
                .ok_or_else(|| "invalid completion edit range".to_string())
        })
        .collect()
}
