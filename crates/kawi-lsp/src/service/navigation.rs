use super::navigation_results::{
    definition_locations, hover_lines, join_hovers, reference_locations, sort_locations,
};
use super::*;
use crate::protocol::{Message, RequestId, ResponseError};
use crate::text;
use crate::workspace;
use lsp_types::{
    GotoDefinitionParams, HoverParams, Position, PositionEncodingKind, ReferenceContext,
    ReferenceParams, TextDocumentIdentifier, TextDocumentPositionParams,
};
use ropey::Rope;
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RequestKind {
    Definition,
    References,
    Hover,
}

pub(super) struct PendingRequest {
    request: u64,
    kind: RequestKind,
    path: PathBuf,
    position: CursorPosition,
    remaining: usize,
    locations: Vec<TextLocation>,
    hovers: Vec<Vec<String>>,
    error: Option<String>,
}

pub(super) struct PendingReply {
    request: u64,
    key: ClientKey,
    encoding: PositionEncodingKind,
}

struct RequestTarget {
    key: ClientKey,
    uri: lsp_types::Uri,
    position: Position,
}

impl Manager {
    pub(super) fn start_definition(
        &mut self,
        request: u64,
        path: PathBuf,
        position: CursorPosition,
    ) {
        self.start_navigation(request, path, position, RequestKind::Definition);
    }

    pub(super) fn start_references(
        &mut self,
        request: u64,
        path: PathBuf,
        position: CursorPosition,
    ) {
        self.start_navigation(request, path, position, RequestKind::References);
    }

    pub(super) fn start_hover(&mut self, request: u64, path: PathBuf, position: CursorPosition) {
        self.start_navigation(request, path, position, RequestKind::Hover);
    }

    fn start_navigation(
        &mut self,
        request: u64,
        path: PathBuf,
        position: CursorPosition,
        kind: RequestKind,
    ) {
        let path = workspace::absolute(&path);
        let Some(text) = self
            .documents
            .get(&path)
            .map(|document| document.text.clone())
        else {
            self.emit(LspEvent::RequestError {
                request,
                message: "document is not open".to_string(),
            });
            return;
        };
        let targets = self.request_targets(&path, kind, &text, position);
        if targets.is_empty() {
            self.emit(LspEvent::RequestError {
                request,
                message: unsupported_message(kind).to_string(),
            });
            return;
        }
        self.pending_requests.insert(
            request,
            PendingRequest {
                request,
                kind,
                path,
                position,
                remaining: targets.len(),
                locations: Vec::new(),
                hovers: Vec::new(),
                error: None,
            },
        );
        for target in targets {
            self.send_navigation_request(request, kind, target);
        }
    }

    fn request_targets(
        &self,
        path: &Path,
        kind: RequestKind,
        text: &Rope,
        position: CursorPosition,
    ) -> Vec<RequestTarget> {
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
                    || !supports(client.support, kind)
                {
                    return None;
                }
                let position = text::position_from_column(
                    text,
                    position.line,
                    position.column,
                    &client.encoding,
                );
                Some(RequestTarget {
                    key: key.clone(),
                    uri: document.uri.clone(),
                    position,
                })
            })
            .collect()
    }

    fn send_navigation_request(&mut self, request: u64, kind: RequestKind, target: RequestTarget) {
        let id = self.request_id();
        let params = navigation_params(kind, target.uri, target.position);
        let message =
            params.map(|params| Message::request(id.clone(), kind.method(), Some(params)));
        self.pending_replies.insert(
            id.clone(),
            PendingReply {
                request,
                key: target.key.clone(),
                encoding: self
                    .clients
                    .get(&target.key)
                    .map_or(PositionEncodingKind::UTF16, |client| {
                        client.encoding.clone()
                    }),
            },
        );
        let Some(message) = message else {
            self.fail_reply(&id, "request params could not be serialized".to_string());
            return;
        };
        let sent = self
            .clients
            .get(&target.key)
            .is_some_and(|client| client.client.send(message));
        if !sent {
            self.fail_reply(&id, "server is not accepting messages".to_string());
        }
    }

    pub(super) fn cancel_navigation(&mut self, request: u64) {
        if self.pending_requests.remove(&request).is_none() {
            return;
        }
        let ids: Vec<RequestId> = self
            .pending_replies
            .iter()
            .filter(|(_, reply)| reply.request == request)
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            let Some(reply) = self.pending_replies.remove(&id) else {
                continue;
            };
            let Some(client) = self.clients.get(&reply.key) else {
                continue;
            };
            client.client.send(Message::notification(
                "$/cancelRequest",
                Some(serde_json::json!({"id": id.to_value()})),
            ));
        }
    }

    pub(super) fn handle_navigation_response(
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
        let Some(group) = self.pending_requests.get_mut(&reply.request) else {
            return;
        };
        match (group.kind, result) {
            (RequestKind::Definition, Ok(value)) => {
                group.locations.extend(definition_locations(
                    &self.documents,
                    value,
                    &reply.encoding,
                ));
            }
            (RequestKind::References, Ok(value)) => {
                group.locations.extend(reference_locations(
                    &self.documents,
                    value,
                    &reply.encoding,
                ));
            }
            (RequestKind::Hover, Ok(value)) => {
                if let Some(lines) = hover_lines(value) {
                    group.hovers.push(lines);
                }
            }
            (_, Err(error)) => {
                if group.error.is_none() {
                    group.error = Some(one_line_message(&error.message));
                }
            }
        }
        self.advance_navigation(reply.request);
    }

    pub(super) fn fail_navigation_for_client(&mut self, key: &ClientKey, message: &str) {
        let ids: Vec<RequestId> = self
            .pending_replies
            .iter()
            .filter(|(_, reply)| &reply.key == key)
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            self.fail_reply(&id, message.to_string());
        }
    }

    fn fail_reply(&mut self, id: &RequestId, message: String) {
        let Some(reply) = self.pending_replies.remove(id) else {
            return;
        };
        let Some(group) = self.pending_requests.get_mut(&reply.request) else {
            return;
        };
        if group.error.is_none() {
            group.error = Some(message);
        }
        self.advance_navigation(reply.request);
    }

    fn advance_navigation(&mut self, request: u64) {
        let Some(group) = self.pending_requests.get_mut(&request) else {
            return;
        };
        group.remaining = group.remaining.saturating_sub(1);
        if group.remaining > 0 {
            return;
        }
        if let Some(group) = self.pending_requests.remove(&request) {
            self.finish_navigation(group);
        }
    }

    fn finish_navigation(&mut self, group: PendingRequest) {
        let PendingRequest {
            request,
            kind,
            path,
            position,
            locations,
            hovers,
            error,
            ..
        } = group;
        match kind {
            RequestKind::Definition | RequestKind::References => {
                let mut locations = locations;
                sort_locations(&mut locations);
                if locations.is_empty() {
                    self.emit(LspEvent::RequestError {
                        request,
                        message: error.unwrap_or_else(|| empty_message(kind).to_string()),
                    });
                } else {
                    let event = LocationEvent {
                        request,
                        path,
                        position,
                        locations,
                    };
                    self.emit(match kind {
                        RequestKind::Definition => LspEvent::Definition(event),
                        _ => LspEvent::References(event),
                    });
                }
            }
            RequestKind::Hover => {
                let contents = join_hovers(hovers);
                if contents.is_empty() {
                    self.emit(LspEvent::RequestError {
                        request,
                        message: error.unwrap_or_else(|| empty_message(kind).to_string()),
                    });
                } else {
                    self.emit(LspEvent::Hover(HoverEvent {
                        request,
                        path,
                        position,
                        contents,
                    }));
                }
            }
        }
    }
}

impl RequestKind {
    fn method(self) -> &'static str {
        match self {
            Self::Definition => "textDocument/definition",
            Self::References => "textDocument/references",
            Self::Hover => "textDocument/hover",
        }
    }
}

fn supports(support: RequestSupport, kind: RequestKind) -> bool {
    match kind {
        RequestKind::Definition => support.definition,
        RequestKind::References => support.references,
        RequestKind::Hover => support.hover,
    }
}

fn unsupported_message(kind: RequestKind) -> &'static str {
    match kind {
        RequestKind::Definition => "No language server supports definitions",
        RequestKind::References => "No language server supports references",
        RequestKind::Hover => "No language server supports hover",
    }
}

fn empty_message(kind: RequestKind) -> &'static str {
    match kind {
        RequestKind::Definition => "No definition found",
        RequestKind::References => "No references found",
        RequestKind::Hover => "No hover information",
    }
}

fn navigation_params(kind: RequestKind, uri: lsp_types::Uri, position: Position) -> Option<Value> {
    let text_document = TextDocumentIdentifier { uri };
    let text_document_position = TextDocumentPositionParams {
        text_document,
        position,
    };
    match kind {
        RequestKind::Definition => to_params(GotoDefinitionParams {
            text_document_position_params: text_document_position,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        }),
        RequestKind::References => to_params(ReferenceParams {
            text_document_position,
            context: ReferenceContext {
                include_declaration: true,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        }),
        RequestKind::Hover => to_params(HoverParams {
            text_document_position_params: text_document_position,
            work_done_progress_params: Default::default(),
        }),
    }
}

fn one_line_message(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}
