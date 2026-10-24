use kawi_lsp::LspService;

use super::LspText;

impl LspText {
    pub(in crate::app) fn cancel_completion(&mut self, service: Option<&LspService>) -> bool {
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

    pub(in crate::app) fn cancel_completion_resolve(
        &mut self,
        service: Option<&LspService>,
    ) -> bool {
        let Some(request) = self.completion_resolve_request.take() else {
            self.completion_resolved = None;
            return false;
        };
        if let Some(service) = service {
            service.cancel(request);
        }
        self.completion_resolved = None;
        self.completion_resolve_failed = None;
        true
    }

    pub(in crate::app) fn cancel_signature(&mut self, service: Option<&LspService>) -> bool {
        let Some(request) = self.signature_request.take() else {
            self.signature = None;
            return false;
        };
        if let Some(service) = service {
            service.cancel(request);
        }
        self.signature = None;
        true
    }

    pub(in crate::app) fn cancel_code_actions(&mut self, service: Option<&LspService>) -> bool {
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

    pub(in crate::app) fn cancel_formatting(&mut self, service: Option<&LspService>) -> bool {
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

    pub(in crate::app) fn cancel_document_requests(
        &mut self,
        service: Option<&LspService>,
    ) -> bool {
        let mut cancelled = self.cancel_completion(service);
        cancelled |= self.cancel_signature(service);
        cancelled |= self.cancel_code_actions(service);
        cancelled |= self.cancel_formatting(service);
        cancelled
    }

    pub(in crate::app) fn cancel_all(&mut self, service: Option<&LspService>) -> bool {
        let requests = [
            self.completion_request.take(),
            self.completion_resolve_request.take(),
            self.signature_request.take(),
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
        self.completion_resolved = None;
        self.signature = None;
        self.code_actions = None;
        self.formatting = None;
        cancelled
    }
}
