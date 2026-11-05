use super::super::*;
use super::convert::{document_colors, merge};
use super::{ColorResponse, PendingColors};
use crate::protocol::{RequestId, ResponseError};
use serde_json::Value;

impl Manager {
    pub(crate) fn handle_colors_response(
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
        let Some(pending) = self.pending_colors.get_mut(&reply.request) else {
            return;
        };
        match result {
            Ok(value) => pending.responses.push(ColorResponse {
                value,
                encoding: reply.encoding,
            }),
            Err(error) => {
                if pending.error.is_none() {
                    pending.error = Some(one_line_message(&error.message));
                }
            }
        }
        self.advance_colors(reply.request);
    }

    pub(crate) fn fail_colors_request(&mut self, request: u64, message: String) {
        let Some(pending) = self.pending_colors.get_mut(&request) else {
            return;
        };
        if pending.error.is_none() {
            pending.error = Some(message);
        }
        self.advance_colors(request);
    }

    fn advance_colors(&mut self, request: u64) {
        let Some(pending) = self.pending_colors.get_mut(&request) else {
            return;
        };
        pending.remaining = pending.remaining.saturating_sub(1);
        if pending.remaining > 0 {
            return;
        }
        let Some(pending) = self.pending_colors.remove(&request) else {
            return;
        };
        self.finish_colors(pending);
    }

    fn finish_colors(&mut self, pending: PendingColors) {
        let Some(document) = self.documents.get(&pending.path) else {
            self.emit_text_error(pending.request, "document is not open".to_string());
            return;
        };
        if document.version != pending.version {
            self.emit_text_error(pending.request, "document changed".to_string());
            return;
        }
        let mut colors = Vec::new();
        let mut parsed = false;
        let mut error = pending.error;
        for response in &pending.responses {
            match document_colors(&response.value, &pending.text, &response.encoding) {
                Ok(mut entries) => {
                    parsed = true;
                    colors.append(&mut entries);
                }
                Err(message) => {
                    if error.is_none() {
                        error = Some(message);
                    }
                }
            }
        }
        if !parsed {
            self.emit_text_error(
                pending.request,
                error.unwrap_or_else(|| "No language server returned colors".to_string()),
            );
            return;
        }
        merge(&mut colors);
        self.emit(LspEvent::DocumentColors(DocumentColorsEvent {
            request: pending.request,
            path: pending.path,
            colors,
        }));
    }
}
