use crossterm::event::{KeyCode, KeyEvent};

use kawi_lsp::{CompletionTrigger, SignatureHelpEvent};

use super::App;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SignatureSession {
    event: SignatureHelpEvent,
}

impl SignatureSession {
    pub(super) fn new(event: SignatureHelpEvent) -> Self {
        Self { event }
    }

    pub(super) fn event(&self) -> &SignatureHelpEvent {
        &self.event
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
