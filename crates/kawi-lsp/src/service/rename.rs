use super::pending::ReplyPurpose;
use super::*;
use crate::protocol::{RequestId, ResponseError};
use crate::text;
use crate::workspace;
use lsp_types::{
    PositionEncodingKind, PrepareRenameResponse, Range, RenameParams, TextDocumentIdentifier,
    TextDocumentPositionParams,
};
use ropey::Rope;
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RenamePhase {
    Preparing,
    Ready,
    Renaming,
}

pub(super) struct PendingRename {
    pub(super) key: ClientKey,
    phase: RenamePhase,
    path: PathBuf,
    position: CursorPosition,
}

impl Manager {
    pub(super) fn start_prepare_rename(
        &mut self,
        request: u64,
        path: PathBuf,
        position: CursorPosition,
    ) {
        let path = workspace::absolute(&path);
        let Some(document) = self.documents.get(&path) else {
            self.emit(LspEvent::RequestError {
                request,
                message: "document is not open".to_string(),
            });
            return;
        };
        let Some(key) = self.rename_client(&path) else {
            self.emit(LspEvent::RequestError {
                request,
                message: "No language server supports rename".to_string(),
            });
            return;
        };
        let uri = document.uri.clone();
        let text = document.text.clone();
        let encoding = self
            .clients
            .get(&key)
            .map_or(PositionEncodingKind::UTF16, |client| {
                client.encoding.clone()
            });
        let prepare = self
            .clients
            .get(&key)
            .is_some_and(|client| client.support.prepare_rename);
        self.pending_renames.insert(
            request,
            PendingRename {
                key,
                phase: if prepare {
                    RenamePhase::Preparing
                } else {
                    RenamePhase::Ready
                },
                path,
                position,
            },
        );
        if prepare {
            self.send_prepare_rename(request, uri, position, encoding);
            return;
        }
        self.emit_local_rename_ready(request, &text, position);
    }

    pub(super) fn start_rename(&mut self, request: u64, new_name: String) {
        let Some(pending) = self.pending_renames.get_mut(&request) else {
            return;
        };
        if pending.phase != RenamePhase::Ready {
            return;
        }
        pending.phase = RenamePhase::Renaming;
        let key = pending.key.clone();
        let position = pending.position;
        let path = pending.path.clone();
        let (uri, document_text, encoding) = {
            let Some(document) = self.documents.get(&path) else {
                self.fail_rename_request(request, "document is not open".to_string());
                return;
            };
            let encoding = self
                .clients
                .get(&key)
                .map_or(PositionEncodingKind::UTF16, |client| {
                    client.encoding.clone()
                });
            (document.uri.clone(), document.text.clone(), encoding)
        };
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: text::position_from_column(
                    &document_text,
                    position.line,
                    position.column,
                    &encoding,
                ),
            },
            new_name,
            work_done_progress_params: Default::default(),
        };
        self.send_tracked_request(
            request,
            &key,
            "textDocument/rename",
            to_params(params),
            ReplyPurpose::Rename,
        );
    }

    pub(super) fn handle_rename_response(
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
        let request = reply.request;
        let encoding = reply.encoding.clone();
        let Some(pending) = self.pending_renames.get(&request) else {
            self.pending_replies.remove(&id);
            return;
        };
        let phase = pending.phase;
        let path = pending.path.clone();
        let position = pending.position;
        self.pending_replies.remove(&id);
        match phase {
            RenamePhase::Preparing => {
                self.finish_prepare(request, &path, position, result, &encoding)
            }
            RenamePhase::Renaming => self.finish_rename(request, result, &encoding),
            RenamePhase::Ready => {}
        }
    }

    fn finish_prepare(
        &mut self,
        request: u64,
        path: &Path,
        position: CursorPosition,
        result: std::result::Result<Value, ResponseError>,
        encoding: &PositionEncodingKind,
    ) {
        let text = self
            .documents
            .get(path)
            .map(|document| document.text.clone());
        let placeholder = match result {
            Err(error) => Err(one_line_message(&error.message)),
            Ok(value) => prepare_placeholder(value, text.as_ref(), position, encoding),
        };
        match placeholder {
            Ok(placeholder) => self.emit_rename_ready(request, path.to_path_buf(), placeholder),
            Err(message) => self.fail_rename_request(request, message),
        }
    }

    fn finish_rename(
        &mut self,
        request: u64,
        result: std::result::Result<Value, ResponseError>,
        encoding: &PositionEncodingKind,
    ) {
        match result {
            Err(error) => self.fail_rename_request(request, one_line_message(&error.message)),
            Ok(value) => match self.parse_workspace_edit(value, encoding) {
                Ok(edit) => {
                    self.pending_renames.remove(&request);
                    self.emit(LspEvent::Rename(RenameEvent { request, edit }));
                }
                Err(message) => self.fail_rename_request(request, message),
            },
        }
    }

    pub(super) fn fail_rename_request(&mut self, request: u64, message: String) {
        if self.pending_renames.remove(&request).is_none() {
            return;
        }
        self.emit(LspEvent::RequestError { request, message });
    }

    fn rename_client(&self, path: &Path) -> Option<ClientKey> {
        let document = self.documents.get(path)?;
        document.clients.iter().find_map(|key| {
            let client = self.clients.get(key)?;
            (client.status == ServerState::Ready
                && client.opened.contains(path)
                && client.support.rename)
                .then_some(key.clone())
        })
    }

    fn send_prepare_rename(
        &mut self,
        request: u64,
        uri: lsp_types::Uri,
        position: CursorPosition,
        encoding: PositionEncodingKind,
    ) {
        let Some(pending) = self.pending_renames.get(&request) else {
            return;
        };
        let key = pending.key.clone();
        let Some(document) = self.documents.get(&pending.path) else {
            self.fail_rename_request(request, "document is not open".to_string());
            return;
        };
        let params = TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri },
            position: text::position_from_column(
                &document.text,
                position.line,
                position.column,
                &encoding,
            ),
        };
        self.send_tracked_request(
            request,
            &key,
            "textDocument/prepareRename",
            to_params(params),
            ReplyPurpose::Rename,
        );
    }

    fn emit_local_rename_ready(&mut self, request: u64, text: &Rope, position: CursorPosition) {
        match local_word(text, position) {
            Some(word) => {
                let path = self
                    .pending_renames
                    .get(&request)
                    .map(|pending| pending.path.clone());
                let Some(path) = path else {
                    return;
                };
                self.emit_rename_ready(request, path, word);
            }
            None => self.fail_rename_request(request, "No symbol under cursor".to_string()),
        }
    }

    fn emit_rename_ready(&mut self, request: u64, path: PathBuf, placeholder: String) {
        let Some(pending) = self.pending_renames.get_mut(&request) else {
            return;
        };
        if pending.path != path {
            return;
        }
        pending.phase = RenamePhase::Ready;
        self.emit(LspEvent::RenameReady(RenameReady {
            request,
            placeholder,
        }));
    }
}

fn prepare_placeholder(
    value: Value,
    text: Option<&Rope>,
    position: CursorPosition,
    encoding: &PositionEncodingKind,
) -> Result<String, String> {
    if value.is_null() {
        return Err("No symbol under cursor".to_string());
    }
    let response = serde_json::from_value::<PrepareRenameResponse>(value)
        .map_err(|error| format!("Invalid prepare rename response: {error}"))?;
    let text = text.ok_or_else(|| "document is not open".to_string())?;
    let placeholder = match response {
        PrepareRenameResponse::RangeWithPlaceholder { placeholder, .. } => placeholder,
        PrepareRenameResponse::Range(range) => range_text(text, range, encoding)?,
        PrepareRenameResponse::DefaultBehavior {
            default_behavior: true,
        } => {
            return local_word(text, position).ok_or_else(|| "No symbol under cursor".to_string());
        }
        PrepareRenameResponse::DefaultBehavior { .. } => {
            return Err("Cannot rename this symbol".to_string());
        }
    };
    if placeholder.is_empty() {
        return Err("No symbol under cursor".to_string());
    }
    Ok(placeholder)
}

fn range_text(
    text: &Rope,
    range: Range,
    encoding: &PositionEncodingKind,
) -> Result<String, String> {
    let start = text::char_offset(text, range.start, encoding)
        .ok_or_else(|| "Prepare rename range is out of bounds".to_string())?;
    let end = text::char_offset(text, range.end, encoding)
        .ok_or_else(|| "Prepare rename range is out of bounds".to_string())?;
    if start > end {
        return Err("Prepare rename range is invalid".to_string());
    }
    Ok(text.slice(start..end).to_string())
}

pub(super) fn local_word(text: &Rope, position: CursorPosition) -> Option<String> {
    let line = usize::try_from(position.line).ok()?;
    if line >= text.len_lines() {
        return None;
    }
    let mut chars: Vec<char> = text.line(line).chars().collect();
    while matches!(chars.last(), Some('\n' | '\r')) {
        chars.pop();
    }
    if chars.is_empty() {
        return None;
    }
    let column = usize::try_from(position.column).unwrap_or(usize::MAX);
    let index = column.min(chars.len().saturating_sub(1));
    if !is_word_char(chars[index]) {
        return None;
    }
    let first = (0..index)
        .rev()
        .find(|position| !is_word_char(chars[*position]))
        .map_or(0, |position| position + 1);
    let last = (index..chars.len())
        .find(|position| !is_word_char(chars[*position]))
        .unwrap_or(chars.len());
    Some(chars[first..last].iter().collect())
}

fn is_word_char(character: char) -> bool {
    character == '_' || character.is_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn position(line: u32, column: u32) -> CursorPosition {
        CursorPosition { line, column }
    }

    #[test]
    fn finds_the_word_under_the_cursor() {
        let text = Rope::from_str("let value_name = other;\n");
        assert_eq!(
            local_word(&text, position(0, 6)),
            Some("value_name".to_string())
        );
        assert_eq!(
            local_word(&text, position(0, 19)),
            Some("other".to_string())
        );
    }

    #[test]
    fn ignores_whitespace_and_missing_lines() {
        let text = Rope::from_str("let value;\n");
        assert_eq!(local_word(&text, position(0, 3)), None);
        assert_eq!(local_word(&text, position(9, 0)), None);
    }
}
