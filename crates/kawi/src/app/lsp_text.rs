use std::path::Path;

use kawi_lsp::{
    CodeActionEvent, CompletionEvent, CursorPosition, Diagnostic, FormattingEvent,
    FormattingSettings, LspService,
};

#[derive(Default)]
pub(super) struct LspText {
    completion_request: Option<u64>,
    completion: Option<CompletionEvent>,
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
    ) -> Result<u64, String> {
        let Some(service) = service else {
            return Err("lsp unavailable".to_string());
        };
        self.cancel_all(Some(service));
        let request = service.completion(path, position);
        self.completion_request = Some(request);
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

    pub(super) fn fail(&mut self, request: u64) -> bool {
        if self.completion_request == Some(request) {
            self.completion_request = None;
            self.completion = None;
            return true;
        }
        if self.code_action_request == Some(request) {
            self.code_action_request = None;
            self.code_actions = None;
            return true;
        }
        if self.formatting_request == Some(request) {
            self.formatting_request = None;
            self.formatting = None;
            return true;
        }
        false
    }

    pub(super) fn cancel_completion(&mut self, service: Option<&LspService>) -> bool {
        let Some(request) = self.completion_request.take() else {
            self.completion = None;
            return false;
        };
        if let Some(service) = service {
            service.cancel(request);
        }
        self.completion = None;
        true
    }

    pub(super) fn cancel_code_actions(&mut self, service: Option<&LspService>) -> bool {
        let Some(request) = self.code_action_request.take() else {
            self.code_actions = None;
            return false;
        };
        if let Some(service) = service {
            service.cancel(request);
        }
        self.code_actions = None;
        true
    }

    pub(super) fn cancel_formatting(&mut self, service: Option<&LspService>) -> bool {
        let Some(request) = self.formatting_request.take() else {
            self.formatting = None;
            return false;
        };
        if let Some(service) = service {
            service.cancel(request);
        }
        self.formatting = None;
        true
    }

    pub(super) fn cancel_all(&mut self, service: Option<&LspService>) -> bool {
        let requests = [
            self.completion_request.take(),
            self.code_action_request.take(),
            self.formatting_request.take(),
        ];
        let mut cancelled = false;
        for request in requests.into_iter().flatten() {
            if let Some(service) = service {
                service.cancel(request);
            }
            cancelled = true;
        }
        self.completion = None;
        self.code_actions = None;
        self.formatting = None;
        cancelled
    }

    pub(super) fn take_completion(&mut self) -> Option<CompletionEvent> {
        self.completion.take()
    }

    pub(super) fn take_code_actions(&mut self) -> Option<CodeActionEvent> {
        self.code_actions.take()
    }

    pub(super) fn take_formatting(&mut self) -> Option<FormattingEvent> {
        self.formatting.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_lsp::{CompletionEntry, TextChange};
    use std::path::PathBuf;

    fn completion(request: u64) -> CompletionEvent {
        CompletionEvent {
            request,
            server: "fake".to_string(),
            path: PathBuf::from("/tmp/lsp_text.rs"),
            version: 1,
            original_hash: 0,
            items: vec![CompletionEntry {
                server: "fake".to_string(),
                label: "value".to_string(),
                detail: None,
                documentation: None,
                primary: TextChange {
                    start: 0,
                    end: 0,
                    text: "value".to_string(),
                },
                additional: Vec::new(),
                deprecated: false,
                preselect: false,
                command: None,
            }],
        }
    }

    #[test]
    fn stale_completion_events_are_ignored() {
        let mut text = LspText {
            completion_request: Some(2),
            ..LspText::default()
        };
        assert!(!text.apply_completion(completion(1)));
        assert!(text.take_completion().is_none());
        assert!(text.apply_completion(completion(2)));
        assert!(text.take_completion().is_some());
    }

    #[test]
    fn failures_clear_the_matching_request() {
        let mut text = LspText {
            code_action_request: Some(3),
            ..LspText::default()
        };
        assert!(text.fail(3));
        assert!(!text.fail(3));
        assert!(text.take_code_actions().is_none());
    }

    #[test]
    fn requests_without_a_service_report_unavailable() {
        let mut text = LspText::default();
        let result = text.completion(
            None,
            Path::new("/tmp/lsp_text.rs"),
            CursorPosition { line: 0, column: 0 },
        );
        assert_eq!(result, Err("lsp unavailable".to_string()));
    }
}
