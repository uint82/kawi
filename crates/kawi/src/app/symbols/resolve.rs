use super::*;
use crate::app::lsp::LspUi;
use kawi_editor::Editor;
use kawi_lsp::{SymbolEntry, SymbolKind};
use kawi_picker::PickerRow;
use std::path::PathBuf;
use std::time::Instant;

pub(super) type Identity = (PathBuf, String, Option<String>, SymbolKind);

pub(super) fn identity(entry: &SymbolEntry) -> Identity {
    (
        entry.path.clone(),
        entry.name.clone(),
        entry.container.clone(),
        entry.kind,
    )
}

pub(super) fn jump_for(entry: &SymbolEntry) -> Option<SymbolJump> {
    let selection = entry.selection?;
    Some(SymbolJump {
        path: entry.path.clone(),
        line: selection.line as usize,
        column: selection.column as usize,
    })
}

impl SymbolSession {
    pub(super) fn open_row(&mut self, row: PickerRow, lsp: &mut LspUi) -> Option<SymbolJump> {
        let id = row.id?;
        if let Some(jump) = self.entries.get(&id).and_then(jump_for) {
            return Some(jump);
        }
        if self.start_resolve(id, true, lsp) {
            return None;
        }
        self.picker.set_status(Some(NO_LOCATION.to_string()));
        None
    }

    pub(super) fn cancel_resolve(&mut self, lsp: &mut LspUi) {
        if let Some(request) = self.resolve_request.take() {
            lsp.cancel_symbol_request(request);
        }
        self.resolve_symbol = None;
        self.jump_after_resolve = false;
    }

    fn start_resolve(&mut self, row_id: u64, jump: bool, lsp: &mut LspUi) -> bool {
        let Some(resolve_id) = self.entries.get(&row_id).and_then(|entry| entry.resolve_id) else {
            return false;
        };
        self.cancel_resolve(lsp);
        match lsp.resolve_workspace_symbol(resolve_id) {
            Ok(request) => {
                self.resolve_request = Some(request);
                self.resolve_symbol = Some(resolve_id);
                self.jump_after_resolve = jump;
                self.picker.set_status(Some(RESOLVING.to_string()));
                self.picker.set_loading(true);
                true
            }
            Err(message) => {
                self.picker.set_status(Some(message));
                false
            }
        }
    }

    pub(super) fn schedule_preview(&mut self, editor: &Editor, lsp: &mut LspUi, now: Instant) {
        let Some(row_id) = self.selected_row().and_then(|row| row.id) else {
            self.preview.clear();
            return;
        };
        let Some(entry) = self.entries.get(&row_id) else {
            self.preview.clear();
            return;
        };
        let path = entry.path.clone();
        let selection = entry.selection;
        let resolve_id = entry.resolve_id;
        if let Some(selection) = selection {
            if self.resolve_symbol.is_some() {
                self.cancel_resolve(lsp);
            }
            let text = editor.file_snapshot(&path);
            self.preview
                .schedule(&path, text, selection.line as usize, now);
            return;
        }
        if let Some(resolve_id) = resolve_id {
            if self.resolve_symbol != Some(resolve_id) {
                self.start_resolve(row_id, false, lsp);
            }
            return;
        }
        self.preview.clear();
        self.picker.set_status(Some(NO_LOCATION.to_string()));
    }
}
