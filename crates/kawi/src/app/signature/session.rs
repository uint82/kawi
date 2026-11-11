use kawi_lsp::SignatureHelpEvent;
use kawi_syntax::LanguageId;
use kawi_tui::markdown::MarkdownDocument;

use super::super::overlay::document::prepare_markup_document;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct SignatureSession {
    event: SignatureHelpEvent,
    documents: Vec<Option<MarkdownDocument>>,
    scroll: usize,
    open: char,
    buffer: usize,
}

impl SignatureSession {
    pub(in crate::app) fn new(
        event: SignatureHelpEvent,
        fallback: Option<LanguageId>,
        open: char,
        buffer: usize,
    ) -> Self {
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
        Self {
            event,
            documents,
            scroll: 0,
            open,
            buffer,
        }
    }

    pub(in crate::app) fn event(&self) -> &SignatureHelpEvent {
        &self.event
    }

    pub(in crate::app) fn document(&self, index: usize) -> Option<&MarkdownDocument> {
        self.documents.get(index)?.as_ref()
    }

    pub(in crate::app) fn scroll(&self) -> usize {
        self.scroll
    }

    pub(in crate::app) fn open(&self) -> char {
        self.open
    }

    pub(in crate::app) fn buffer(&self) -> usize {
        self.buffer
    }

    pub(in crate::app) fn scroll_by(&mut self, delta: isize, limit: usize) {
        self.scroll = self.scroll.saturating_add_signed(delta).min(limit);
    }
}
