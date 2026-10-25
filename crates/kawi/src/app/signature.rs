use crossterm::event::{KeyCode, KeyEvent};

use kawi_lsp::{CompletionTrigger, SignatureHelpEvent};
use kawi_syntax::LanguageId;
use kawi_tui::markdown::MarkdownDocument;

use super::overlay::document::prepare_markup_document;

use super::App;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SignatureSession {
    event: SignatureHelpEvent,
    documents: Vec<Option<MarkdownDocument>>,
}

impl SignatureSession {
    pub(super) fn new(event: SignatureHelpEvent, fallback: Option<LanguageId>) -> Self {
        let documents = event
            .signatures
            .iter()
            .map(|signature| {
                signature
                    .documentation
                    .as_ref()
                    .map(|documentation| prepare_markup_document(documentation, fallback))
            })
            .collect();
        Self { event, documents }
    }

    pub(super) fn event(&self) -> &SignatureHelpEvent {
        &self.event
    }

    pub(super) fn document(&self, index: usize) -> Option<&MarkdownDocument> {
        self.documents.get(index)?.as_ref()
    }
}

impl App {
    pub(super) fn after_signature_key(&mut self, key: KeyEvent) {
        if key.modifiers.intersects(
            crossterm::event::KeyModifiers::CONTROL | crossterm::event::KeyModifiers::ALT,
        ) {
            return;
        }
        if key.code == KeyCode::Char(')') {
            self.signature = None;
            self.lsp.cancel_signature();
            return;
        }
        if !self.completion_options.signature || !self.editor.mode().is_insert() {
            return;
        }
        let KeyCode::Char(character) = key.code else {
            return;
        };
        let capabilities = self.lsp.completion_capabilities();
        if !capabilities.signature_help
            || !capabilities
                .signature_triggers
                .iter()
                .any(|trigger| trigger == &character.to_string())
        {
            return;
        }
        if self
            .lsp
            .signature_help(&self.editor, CompletionTrigger::character(character))
            .is_err()
        {
            self.signature = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_lsp::{MarkupDocument, SignatureEntry};
    use kawi_tui::markdown::{self, MarkdownStyle};

    #[test]
    fn signature_documentation_is_prepared_with_the_buffer_language() {
        let session = SignatureSession::new(
            SignatureHelpEvent {
                request: 1,
                server: "test".to_string(),
                signatures: vec![SignatureEntry {
                    label: "run()".to_string(),
                    documentation: Some(MarkupDocument {
                        text: "```\nfn run() {}\n```".to_string(),
                        markdown: true,
                    }),
                    parameters: Vec::new(),
                    active_parameter: None,
                }],
                active_signature: Some(0),
                active_parameter: None,
            },
            Some(LanguageId::Rust),
        );
        let lines = markdown::render_document(session.document(0).expect("document"), 80);
        assert!(lines[0]
            .spans
            .iter()
            .any(|span| matches!(span.style, MarkdownStyle::Syntax(_))));
    }
}
