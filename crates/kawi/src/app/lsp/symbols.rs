use super::*;
use kawi_lsp::{
    DocumentSymbolEvent, SymbolCapabilitiesEvent, WorkspaceSymbolEvent,
    WorkspaceSymbolResolvedEvent,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SymbolEvent {
    Document(DocumentSymbolEvent),
    Workspace(WorkspaceSymbolEvent),
    Resolved(WorkspaceSymbolResolvedEvent),
    Error { request: u64, message: String },
}

#[derive(Default)]
pub(super) struct LspSymbols {
    capabilities: BTreeMap<String, SymbolCapabilitiesEvent>,
    requests: BTreeSet<u64>,
    events: VecDeque<SymbolEvent>,
}

impl LspSymbols {
    pub(super) fn apply_capabilities(&mut self, event: SymbolCapabilitiesEvent) {
        self.capabilities.insert(event.server.clone(), event);
    }

    pub(super) fn push_document(&mut self, event: DocumentSymbolEvent) {
        self.requests.remove(&event.request);
        self.events.push_back(SymbolEvent::Document(event));
    }

    pub(super) fn push_workspace(&mut self, event: WorkspaceSymbolEvent) {
        self.requests.remove(&event.request);
        self.events.push_back(SymbolEvent::Workspace(event));
    }

    pub(super) fn push_resolved(&mut self, event: WorkspaceSymbolResolvedEvent) {
        self.requests.remove(&event.request);
        self.events.push_back(SymbolEvent::Resolved(event));
    }

    pub(super) fn fail(&mut self, request: u64, message: &str) -> bool {
        if !self.requests.remove(&request) {
            return false;
        }
        self.events.push_back(SymbolEvent::Error {
            request,
            message: message.to_string(),
        });
        true
    }

    pub(super) fn start(&mut self, request: u64) {
        self.requests.insert(request);
    }

    pub(super) fn cancel(&mut self, request: u64, service: Option<&LspService>) {
        self.requests.remove(&request);
        if let Some(service) = service {
            service.cancel(request);
        }
    }

    pub(super) fn cancel_all(&mut self, service: Option<&LspService>) -> bool {
        let requests: Vec<u64> = self.requests.iter().copied().collect();
        self.requests.clear();
        for request in &requests {
            if let Some(service) = service {
                service.cancel(*request);
            }
        }
        !requests.is_empty()
    }

    pub(super) fn take_event(&mut self) -> Option<SymbolEvent> {
        self.events.pop_front()
    }

    fn supports_document_for(&self, server: &str) -> bool {
        self.capabilities
            .get(server)
            .is_some_and(|capabilities| capabilities.document)
    }

    fn supports_workspace(&self) -> bool {
        self.capabilities
            .values()
            .any(|capabilities| capabilities.workspace)
    }
}

impl LspUi {
    pub fn document_symbols(&mut self, editor: &Editor, version: u64) -> Result<u64, String> {
        self.flush_pending_change();
        let Some(service) = self.service.as_ref() else {
            return Err("lsp unavailable".to_string());
        };
        let Some(path) = editor_path(editor) else {
            return Err("E32: No file name".to_string());
        };
        if !self.supports_document_symbols() {
            return Err("No language server supports document symbols".to_string());
        }
        let request = service.document_symbols(&path, version);
        self.symbols.start(request);
        Ok(request)
    }

    pub fn workspace_symbols(&mut self, query: &str) -> Result<u64, String> {
        self.flush_pending_change();
        let Some(service) = self.service.as_ref() else {
            return Err("lsp unavailable".to_string());
        };
        if !self.supports_workspace_symbols() {
            return Err("No language server supports workspace symbols".to_string());
        }
        let request = service.workspace_symbols(query);
        self.symbols.start(request);
        Ok(request)
    }

    pub fn resolve_workspace_symbol(&mut self, symbol_id: u64) -> Result<u64, String> {
        self.flush_pending_change();
        let Some(service) = self.service.as_ref() else {
            return Err("lsp unavailable".to_string());
        };
        let request = service.resolve_workspace_symbol(symbol_id);
        self.symbols.start(request);
        Ok(request)
    }

    pub fn take_symbol_event(&mut self) -> Option<SymbolEvent> {
        self.symbols.take_event()
    }

    pub fn cancel_symbol_request(&mut self, request: u64) {
        self.symbols.cancel(request, self.service.as_ref());
    }

    pub fn supports_document_symbols(&self) -> bool {
        let Some(path) = self.path.as_deref() else {
            return false;
        };
        let Some(language) = self
            .languages
            .as_ref()
            .and_then(|languages| languages.language_for_path(path))
        else {
            return false;
        };
        language
            .servers
            .iter()
            .any(|server| self.symbols.supports_document_for(server))
    }

    pub fn supports_workspace_symbols(&self) -> bool {
        self.symbols.supports_workspace()
    }
}
