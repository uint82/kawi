use super::super::pending::ReplyPurpose;
use super::super::*;
use super::PendingColors;
use crate::workspace;
use lsp_types::{DocumentColorParams, TextDocumentIdentifier};

impl Manager {
    pub(crate) fn start_document_colors(&mut self, request: u64, path: PathBuf) {
        let path = workspace::absolute(&path);
        let Some(document) = self.documents.get(&path) else {
            self.emit_text_error(request, "document is not open".to_string());
            return;
        };
        let targets: Vec<(ClientKey, lsp_types::Uri)> = document
            .clients
            .iter()
            .filter_map(|key| {
                let client = self.clients.get(key)?;
                (client.status == ServerState::Ready
                    && client.opened.contains(&path)
                    && client.support.color_provider)
                    .then(|| (key.clone(), document.uri.clone()))
            })
            .collect();
        if targets.is_empty() {
            self.emit_text_error(
                request,
                "No language server supports document colors".to_string(),
            );
            return;
        }
        let text = document.text.clone();
        let version = document.version;
        self.pending_colors.insert(
            request,
            PendingColors {
                request,
                path,
                version,
                text,
                remaining: targets.len(),
                responses: Vec::new(),
                error: None,
            },
        );
        for (key, uri) in targets {
            let params = DocumentColorParams {
                text_document: TextDocumentIdentifier { uri },
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
            };
            self.send_tracked_request(
                request,
                &key,
                "textDocument/documentColor",
                to_params(params),
                ReplyPurpose::Colors,
            );
        }
    }

    pub(crate) fn cancel_color_request(&mut self, request: u64) -> bool {
        self.pending_colors.remove(&request).is_some()
    }
}
