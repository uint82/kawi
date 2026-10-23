use super::code_actions::protocol_diagnostics;
use super::completion::completion_event;
use super::pending::ReplyPurpose;
use super::*;
use crate::protocol::{RequestId, ResponseError};
use crate::text;
use crate::workspace;
use lsp_types::{
    CodeActionContext, CodeActionParams, CodeActionTriggerKind, CompletionContext,
    CompletionParams, CompletionTriggerKind, DocumentFormattingParams,
    DocumentRangeFormattingParams, FormattingOptions, Range, TextDocumentIdentifier,
    TextDocumentPositionParams,
};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TextRequestKind {
    Completion,
    CodeAction,
    Formatting,
}

#[derive(Debug, Clone)]
pub(super) struct PendingTextRequest {
    pub(super) request: u64,
    pub(super) kind: TextRequestKind,
    pub(super) path: PathBuf,
    pub(super) position: CursorPosition,
    pub(super) version: i32,
    pub(super) remaining: usize,
    pub(super) responses: Vec<TextResponse>,
    pub(super) error: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct TextResponse {
    pub(super) server: String,
    pub(super) value: Value,
    pub(super) encoding: PositionEncodingKind,
}

impl PendingTextRequest {
    fn new(
        request: u64,
        kind: TextRequestKind,
        path: PathBuf,
        position: CursorPosition,
        version: i32,
        targets: usize,
    ) -> Self {
        Self {
            request,
            kind,
            path,
            position,
            version,
            remaining: targets,
            responses: Vec::new(),
            error: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextCapability {
    Completion,
    CodeAction,
    Formatting,
    RangeFormatting,
}

struct TextTarget {
    key: ClientKey,
    uri: lsp_types::Uri,
    text: Rope,
    version: i32,
    encoding: PositionEncodingKind,
}

impl Manager {
    pub(super) fn start_completion(
        &mut self,
        request: u64,
        path: PathBuf,
        position: CursorPosition,
    ) {
        let path = workspace::absolute(&path);
        let targets = self.text_targets(&path, TextCapability::Completion);
        let Some(version) = targets.first().map(|target| target.version) else {
            self.emit_unsupported(request, &path, TextCapability::Completion);
            return;
        };
        self.pending_text_requests.insert(
            request,
            PendingTextRequest::new(
                request,
                TextRequestKind::Completion,
                path,
                position,
                version,
                targets.len(),
            ),
        );
        for target in targets {
            let protocol_position = text::position_from_column(
                &target.text,
                position.line,
                position.column,
                &target.encoding,
            );
            let params = CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: target.uri.clone(),
                    },
                    position: protocol_position,
                },
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
                context: Some(CompletionContext {
                    trigger_kind: CompletionTriggerKind::INVOKED,
                    trigger_character: None,
                }),
            };
            self.dispatch_text_request(
                request,
                &target.key,
                "textDocument/completion",
                to_params(params),
            );
        }
    }

    pub(super) fn start_code_actions(
        &mut self,
        request: u64,
        path: PathBuf,
        range: (CursorPosition, CursorPosition),
        diagnostics: Vec<Diagnostic>,
    ) {
        let path = workspace::absolute(&path);
        let targets = self.text_targets(&path, TextCapability::CodeAction);
        let Some(version) = targets.first().map(|target| target.version) else {
            self.emit_unsupported(request, &path, TextCapability::CodeAction);
            return;
        };
        self.pending_text_requests.insert(
            request,
            PendingTextRequest::new(
                request,
                TextRequestKind::CodeAction,
                path,
                range.0,
                version,
                targets.len(),
            ),
        );
        for target in targets {
            let protocol_range = protocol_range(&target.text, range, &target.encoding);
            let params = CodeActionParams {
                text_document: TextDocumentIdentifier {
                    uri: target.uri.clone(),
                },
                range: protocol_range,
                context: CodeActionContext {
                    diagnostics: protocol_diagnostics(&diagnostics, &target.text, &target.encoding),
                    only: None,
                    trigger_kind: Some(CodeActionTriggerKind::INVOKED),
                },
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
            };
            self.dispatch_text_request(
                request,
                &target.key,
                "textDocument/codeAction",
                to_params(params),
            );
        }
    }

    pub(super) fn start_formatting(
        &mut self,
        request: u64,
        path: PathBuf,
        range: Option<(CursorPosition, CursorPosition)>,
        settings: FormattingSettings,
    ) {
        let path = workspace::absolute(&path);
        let capability = if range.is_some() {
            TextCapability::RangeFormatting
        } else {
            TextCapability::Formatting
        };
        let Some(target) = self.text_targets(&path, capability).into_iter().next() else {
            self.emit_unsupported(request, &path, capability);
            return;
        };
        let version = target.version;
        let options = FormattingOptions {
            tab_size: settings.tab_size,
            insert_spaces: settings.insert_spaces,
            ..FormattingOptions::default()
        };
        let position = range.map_or(CursorPosition { line: 0, column: 0 }, |value| value.0);
        let (method, params) = match range {
            Some(range) => (
                "textDocument/rangeFormatting",
                to_params(DocumentRangeFormattingParams {
                    text_document: TextDocumentIdentifier {
                        uri: target.uri.clone(),
                    },
                    range: protocol_range(&target.text, range, &target.encoding),
                    options,
                    work_done_progress_params: Default::default(),
                }),
            ),
            None => (
                "textDocument/formatting",
                to_params(DocumentFormattingParams {
                    text_document: TextDocumentIdentifier {
                        uri: target.uri.clone(),
                    },
                    options,
                    work_done_progress_params: Default::default(),
                }),
            ),
        };
        self.pending_text_requests.insert(
            request,
            PendingTextRequest::new(
                request,
                TextRequestKind::Formatting,
                path,
                position,
                version,
                1,
            ),
        );
        self.dispatch_text_request(request, &target.key, method, params);
    }

    pub(super) fn handle_text_response(
        &mut self,
        key: ClientKey,
        id: RequestId,
        result: std::result::Result<Value, ResponseError>,
    ) {
        let Some(reply) = self.pending_replies.remove(&id) else {
            return;
        };
        if reply.key != key {
            return;
        }
        let request = reply.request;
        let response = result.map(|value| TextResponse {
            server: reply.key.server.clone(),
            value,
            encoding: reply.encoding,
        });
        let Some(pending) = self.pending_text_requests.get_mut(&request) else {
            return;
        };
        match response {
            Ok(response) => pending.responses.push(response),
            Err(error) => {
                if pending.error.is_none() {
                    pending.error = Some(one_line_message(&error.message));
                }
            }
        }
        self.advance_text_request(request);
    }

    pub(super) fn fail_text_request(&mut self, request: u64, message: String) {
        let Some(pending) = self.pending_text_requests.get_mut(&request) else {
            return;
        };
        if pending.error.is_none() {
            pending.error = Some(message);
        }
        self.advance_text_request(request);
    }

    fn dispatch_text_request(
        &mut self,
        request: u64,
        key: &ClientKey,
        method: &str,
        params: Option<Value>,
    ) {
        self.send_tracked_request(request, key, method, params, ReplyPurpose::Text);
    }

    fn advance_text_request(&mut self, request: u64) {
        let Some(pending) = self.pending_text_requests.get_mut(&request) else {
            return;
        };
        pending.remaining = pending.remaining.saturating_sub(1);
        if pending.remaining > 0 {
            return;
        }
        let Some(pending) = self.pending_text_requests.remove(&request) else {
            return;
        };
        self.finish_text_request(pending);
    }

    fn finish_text_request(&mut self, pending: PendingTextRequest) {
        let Some(document) = self.documents.get(&pending.path) else {
            self.emit_text_error(pending.request, "document is not open".to_string());
            return;
        };
        if document.version != pending.version {
            self.emit_text_error(pending.request, "document changed".to_string());
            return;
        }
        let parse = match pending.kind {
            TextRequestKind::Completion => {
                completion_event(&pending, &pending.responses, &document.text)
                    .map(LspEvent::Completion)
            }
            TextRequestKind::CodeAction => self
                .code_action_event(&pending, &pending.responses)
                .map(LspEvent::CodeActions),
            TextRequestKind::Formatting => self
                .formatting_event(&pending, &pending.responses, &document.text)
                .map(LspEvent::Formatting),
        };
        match parse {
            Ok(event) => self.emit(event),
            Err(error) => self.emit_text_error(pending.request, pending.error.unwrap_or(error)),
        }
    }

    fn text_targets(&self, path: &Path, capability: TextCapability) -> Vec<TextTarget> {
        let Some(document) = self.documents.get(path) else {
            return Vec::new();
        };
        document
            .clients
            .iter()
            .filter_map(|key| {
                let client = self.clients.get(key)?;
                if client.status != ServerState::Ready
                    || !client.opened.contains(path)
                    || !supports(client.support, capability)
                {
                    return None;
                }
                Some(TextTarget {
                    key: key.clone(),
                    uri: document.uri.clone(),
                    text: document.text.clone(),
                    version: document.version,
                    encoding: client.encoding.clone(),
                })
            })
            .collect()
    }

    fn emit_unsupported(&self, request: u64, path: &Path, capability: TextCapability) {
        let message = if self.documents.contains_key(path) {
            unsupported_message(capability).to_string()
        } else {
            "document is not open".to_string()
        };
        self.emit_text_error(request, message);
    }

    fn emit_text_error(&self, request: u64, message: String) {
        self.emit(LspEvent::RequestError { request, message });
    }
}

fn supports(support: RequestSupport, capability: TextCapability) -> bool {
    match capability {
        TextCapability::Completion => support.completion,
        TextCapability::CodeAction => support.code_actions,
        TextCapability::Formatting => support.formatting,
        TextCapability::RangeFormatting => support.range_formatting,
    }
}

fn unsupported_message(capability: TextCapability) -> &'static str {
    match capability {
        TextCapability::Completion => "No language server supports completion",
        TextCapability::CodeAction => "No language server supports code actions",
        TextCapability::Formatting => "No language server supports formatting",
        TextCapability::RangeFormatting => "No language server supports range formatting",
    }
}

fn protocol_range(
    text: &Rope,
    range: (CursorPosition, CursorPosition),
    encoding: &PositionEncodingKind,
) -> Range {
    let (start, end) = normalized_range(range);
    Range::new(
        text::position_from_column(text, start.line, start.column, encoding),
        text::position_from_column(text, end.line, end.column, encoding),
    )
}

fn normalized_range(range: (CursorPosition, CursorPosition)) -> (CursorPosition, CursorPosition) {
    if range.0.line > range.1.line
        || (range.0.line == range.1.line && range.0.column > range.1.column)
    {
        (range.1, range.0)
    } else {
        range
    }
}
