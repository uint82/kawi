use std::path::Path;

mod cancel;

#[cfg(test)]
mod tests;

use kawi_lsp::{
    CodeActionEvent, CompletionEvent, CompletionResolveEvent, CompletionTrigger, CursorPosition,
    Diagnostic, FormattingEvent, FormattingSettings, LspService, SignatureHelpEvent,
    SignatureTrigger,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TextFailure {
    Report,
    Silent,
}

#[derive(Default)]
pub(super) struct LspText {
    completion_request: Option<u64>,
    completion_report_errors: bool,
    completion: Option<CompletionEvent>,
    completion_resolve_request: Option<u64>,
    completion_resolved: Option<CompletionResolveEvent>,
    completion_resolve_failed: Option<u64>,
    signature_request: Option<u64>,
    signature: Option<SignatureHelpEvent>,
    signature_failed: Option<u64>,
    code_action_request: Option<u64>,
    code_actions: Option<CodeActionEvent>,
    formatting_request: Option<u64>,
    formatting: Option<FormattingEvent>,
}

impl LspText {
    pub(super) fn completion(
        &mut self,
        service: Option<&LspService>,
        path: &Path,
        position: CursorPosition,
        trigger: CompletionTrigger,
    ) -> Result<u64, String> {
        let Some(service) = service else {
            return Err("lsp unavailable".to_string());
        };
        self.cancel_completion(Some(service));
        self.cancel_completion_resolve(Some(service));
        let request = service.completion(path, position, trigger);
        self.completion_request = Some(request);
        self.completion_report_errors = true;
        Ok(request)
    }

    pub(super) fn resolve_completion(
        &mut self,
        service: Option<&LspService>,
        item_id: u64,
    ) -> Result<u64, String> {
        let Some(service) = service else {
            return Err("lsp unavailable".to_string());
        };
        if let Some(request) = self.completion_resolve_request.take() {
            service.cancel(request);
        }
        let request = service.resolve_completion(item_id);
        self.completion_resolve_request = Some(request);
        self.completion_resolved = None;
        self.completion_resolve_failed = None;
        Ok(request)
    }

    pub(super) fn signature_help(
        &mut self,
        service: Option<&LspService>,
        path: &Path,
        position: CursorPosition,
        trigger: SignatureTrigger,
    ) -> Result<u64, String> {
        let Some(service) = service else {
            return Err("lsp unavailable".to_string());
        };
        if let Some(request) = self.signature_request.take() {
            service.cancel(request);
        }
        let request = service.signature_help(path, position, trigger);
        self.signature_request = Some(request);
        self.signature = None;
        self.signature_failed = None;
        Ok(request)
    }

    pub(super) fn code_actions(
        &mut self,
        service: Option<&LspService>,
        path: &Path,
        range: (CursorPosition, CursorPosition),
        diagnostics: Vec<Diagnostic>,
    ) -> Result<u64, String> {
        let Some(service) = service else {
            return Err("lsp unavailable".to_string());
        };
        self.cancel_all(Some(service));
        let request = service.code_actions(path, range, diagnostics);
        self.code_action_request = Some(request);
        Ok(request)
    }

    pub(super) fn formatting(
        &mut self,
        service: Option<&LspService>,
        path: &Path,
        range: Option<(CursorPosition, CursorPosition)>,
        settings: FormattingSettings,
    ) -> Result<u64, String> {
        let Some(service) = service else {
            return Err("lsp unavailable".to_string());
        };
        self.cancel_all(Some(service));
        let request = service.formatting(path, range, settings);
        self.formatting_request = Some(request);
        Ok(request)
    }

    pub(super) fn apply_completion(&mut self, event: CompletionEvent) -> bool {
        if self.completion_request != Some(event.request) {
            return false;
        }
        self.completion_request = None;
        self.completion = Some(event);
        true
    }

    pub(super) fn apply_completion_resolved(&mut self, event: CompletionResolveEvent) -> bool {
        if self.completion_resolve_request != Some(event.request) {
            return false;
        }
        self.completion_resolve_request = None;
        self.completion_resolved = Some(event);
        true
    }

    pub(super) fn apply_signature(&mut self, event: SignatureHelpEvent) -> bool {
        if self.signature_request != Some(event.request) {
            return false;
        }
        self.signature_request = None;
        self.signature = Some(event);
        self.signature_failed = None;
        true
    }

    pub(super) fn apply_code_actions(&mut self, event: CodeActionEvent) -> bool {
        if self.code_action_request != Some(event.request) {
            return false;
        }
        self.code_action_request = None;
        self.code_actions = Some(event);
        true
    }

    pub(super) fn formatting_pending(&self, request: u64) -> bool {
        self.formatting_request == Some(request)
    }

    pub(super) fn apply_formatting(&mut self, event: FormattingEvent) -> bool {
        if self.formatting_request != Some(event.request) {
            return false;
        }
        self.formatting_request = None;
        self.formatting = Some(event);
        true
    }

    pub(super) fn fail(&mut self, request: u64) -> Option<TextFailure> {
        if self.completion_request == Some(request) {
            self.completion_request = None;
            self.completion = None;
            return Some(if self.completion_report_errors {
                TextFailure::Report
            } else {
                TextFailure::Silent
            });
        }
        if self.completion_resolve_request == Some(request) {
            self.completion_resolve_request = None;
            self.completion_resolved = None;
            self.completion_resolve_failed = Some(request);
            return Some(TextFailure::Silent);
        }
        if self.signature_request == Some(request) {
            self.signature_request = None;
            self.signature = None;
            self.signature_failed = Some(request);
            return Some(TextFailure::Silent);
        }
        if self.code_action_request == Some(request) {
            self.code_action_request = None;
            self.code_actions = None;
            return Some(TextFailure::Report);
        }
        if self.formatting_request == Some(request) {
            self.formatting_request = None;
            self.formatting = None;
            return Some(TextFailure::Report);
        }
        None
    }

    pub(super) fn set_completion_report_errors(&mut self, request: u64, report: bool) {
        if self.completion_request == Some(request) {
            self.completion_report_errors = report;
        }
    }

    pub(super) fn take_completion(&mut self) -> Option<CompletionEvent> {
        self.completion.take()
    }

    pub(super) fn take_completion_resolved(&mut self) -> Option<CompletionResolveEvent> {
        self.completion_resolved.take()
    }

    pub(super) fn take_completion_resolve_failed(&mut self) -> Option<u64> {
        self.completion_resolve_failed.take()
    }

    pub(super) fn take_signature(&mut self) -> Option<SignatureHelpEvent> {
        self.signature.take()
    }

    pub(super) fn take_signature_failed(&mut self) -> Option<u64> {
        self.signature_failed.take()
    }

    pub(super) fn take_code_actions(&mut self) -> Option<CodeActionEvent> {
        self.code_actions.take()
    }

    pub(super) fn take_formatting(&mut self) -> Option<FormattingEvent> {
        self.formatting.take()
    }
}
