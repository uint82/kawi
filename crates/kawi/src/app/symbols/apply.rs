use super::resolve::{identity, Identity};
use super::rows;
use super::*;
use crate::app::lsp::LspUi;
use kawi_editor::Editor;
use kawi_lsp::{
    DocumentSymbolEvent, SymbolEntry, WorkspaceSymbolEvent, WorkspaceSymbolResolvedEvent,
};
use std::time::Instant;

impl SymbolSession {
    pub(super) fn set_document_symbols(
        &mut self,
        symbols: Vec<SymbolEntry>,
        cursor: (usize, usize),
        now: Instant,
        editor: &Editor,
        lsp: &mut LspUi,
    ) -> bool {
        self.load_entries(symbols);
        self.rebuild_rows(None);
        self.picker.set_loading(false);
        if self.picker.query().is_empty() {
            self.select_innermost(cursor);
        }
        self.schedule_preview(editor, lsp, now);
        true
    }

    pub(super) fn apply_document(
        &mut self,
        event: DocumentSymbolEvent,
        cursor: (usize, usize),
        now: Instant,
        editor: &Editor,
        lsp: &mut LspUi,
    ) -> bool {
        if self.document_request != Some(event.request) {
            return false;
        }
        self.document_request = None;
        self.set_document_symbols(event.symbols, cursor, now, editor, lsp)
    }

    pub(super) fn apply_workspace(
        &mut self,
        event: WorkspaceSymbolEvent,
        now: Instant,
        editor: &Editor,
        lsp: &mut LspUi,
    ) -> bool {
        if self.workspace_request != Some(event.request) {
            return false;
        }
        self.workspace_request = None;
        self.picker.set_loading(false);
        self.picker.set_status(None);
        let selected = self.selected_identity();
        self.load_entries(event.symbols);
        self.rebuild_rows(selected);
        self.schedule_preview(editor, lsp, now);
        true
    }

    pub(super) fn apply_resolved(
        &mut self,
        event: WorkspaceSymbolResolvedEvent,
        now: Instant,
        editor: &Editor,
        lsp: &mut LspUi,
    ) -> Option<SymbolJump> {
        if self.resolve_request != Some(event.request) {
            return None;
        }
        self.resolve_request = None;
        self.resolve_symbol = None;
        let row_id = self.resolve_to_row.remove(&event.symbol_id)?;
        let jump = self
            .jump_after_resolve
            .then(|| super::resolve::jump_for(&event.symbol))
            .flatten();
        self.jump_after_resolve = false;
        let identity = identity(&event.symbol);
        self.entries.insert(row_id, event.symbol);
        self.rebuild_rows(Some(identity));
        self.picker.set_loading(false);
        self.picker.set_status(None);
        if jump.is_none() {
            self.schedule_preview(editor, lsp, now);
        }
        jump
    }

    pub(super) fn apply_error(&mut self, request: u64, message: &str) -> bool {
        if self.document_request == Some(request) {
            self.document_request = None;
        } else if self.workspace_request == Some(request) {
            self.workspace_request = None;
        } else if self.resolve_request == Some(request) {
            self.resolve_request = None;
            self.resolve_symbol = None;
            self.jump_after_resolve = false;
        } else {
            return false;
        }
        self.picker.set_loading(false);
        self.picker.set_status(Some(message.to_string()));
        true
    }

    fn select_innermost(&mut self, cursor: (usize, usize)) {
        if let Some(id) = rows::innermost(&self.entries, cursor.0, cursor.1) {
            self.picker.select_symbol(id);
        }
    }

    fn load_entries(&mut self, symbols: Vec<SymbolEntry>) {
        self.entries.clear();
        self.resolve_to_row.clear();
        for symbol in symbols {
            self.next_id = self.next_id.saturating_add(1);
            let id = self.next_id;
            if let Some(resolve_id) = symbol.resolve_id {
                self.resolve_to_row.insert(resolve_id, id);
            }
            self.entries.insert(id, symbol);
        }
    }

    fn rebuild_rows(&mut self, selection: Option<Identity>) {
        let rows = rows::picker_rows(&self.entries, &self.root);
        if self.mode == SymbolMode::Workspace {
            self.picker.set_workspace_symbols(rows);
        } else {
            self.picker.set_locations(rows);
        }
        if let Some(selection) = selection {
            if let Some(id) = self
                .entries
                .iter()
                .find(|(_, entry)| identity(entry) == selection)
                .map(|(id, _)| *id)
            {
                self.picker.select_symbol(id);
            }
        }
    }
}
