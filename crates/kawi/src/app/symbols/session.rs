use super::preview::PreviewState;
use super::resolve::{identity, Identity};
use super::*;
use crate::app::lsp::LspUi;
use crossterm::event::KeyEvent;
use kawi_editor::Editor;
use kawi_lsp::SymbolEntry;
use kawi_picker::{Picker, PickerOutcome, PickerRow};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

pub(crate) struct SymbolSession {
    pub(super) picker: Picker,
    pub(super) root: PathBuf,
    pub(super) mode: SymbolMode,
    pub(super) entries: BTreeMap<u64, SymbolEntry>,
    pub(super) resolve_to_row: BTreeMap<u64, u64>,
    pub(super) next_id: u64,
    pub(super) document_request: Option<u64>,
    pub(super) workspace_request: Option<u64>,
    pub(super) workspace_deadline: Option<Instant>,
    pub(super) resolve_request: Option<u64>,
    pub(super) resolve_symbol: Option<u64>,
    pub(super) jump_after_resolve: bool,
    pub(super) preview: PreviewState,
}

impl SymbolSession {
    pub(super) fn document(root: PathBuf, wake: Sender<()>) -> Self {
        let mut picker = Picker::symbols("Symbols", Vec::new());
        picker.set_loading(true);
        Self::new(picker, root, SymbolMode::Document, wake)
    }

    pub(super) fn workspace(root: PathBuf, wake: Sender<()>) -> Self {
        Self::new(
            Picker::workspace_symbols("Workspace symbols"),
            root,
            SymbolMode::Workspace,
            wake,
        )
    }

    fn new(picker: Picker, root: PathBuf, mode: SymbolMode, wake: Sender<()>) -> Self {
        let preview = PreviewState::new(root.clone(), wake);
        Self {
            picker,
            root,
            mode,
            entries: BTreeMap::new(),
            resolve_to_row: BTreeMap::new(),
            next_id: 0,
            document_request: None,
            workspace_request: None,
            workspace_deadline: None,
            resolve_request: None,
            resolve_symbol: None,
            jump_after_resolve: false,
            preview,
        }
    }

    pub(crate) fn picker(&self) -> &Picker {
        &self.picker
    }

    pub(super) fn set_document_request(&mut self, request: u64) {
        self.document_request = Some(request);
    }

    pub(super) fn handle_key(
        &mut self,
        key: KeyEvent,
        editor: &Editor,
        lsp: &mut LspUi,
        now: Instant,
    ) -> SymbolKeyOutcome {
        let selected = self.selected_row().and_then(|row| row.id);
        match self.picker.handle_key(key) {
            PickerOutcome::Cancel => {
                self.close(lsp);
                return SymbolKeyOutcome::Closed;
            }
            PickerOutcome::Open(row) => {
                if let Some(jump) = self.open_row(row, lsp) {
                    return SymbolKeyOutcome::Jump(jump);
                }
            }
            PickerOutcome::QueryChanged => self.query_changed(lsp, now),
            PickerOutcome::Consumed => {}
        }
        if self.selected_row().and_then(|row| row.id) != selected {
            self.schedule_preview(editor, lsp, now);
        }
        SymbolKeyOutcome::Consumed
    }

    fn query_changed(&mut self, lsp: &mut LspUi, now: Instant) {
        if self.mode != SymbolMode::Workspace {
            return;
        }
        self.cancel_workspace_request(lsp);
        self.cancel_resolve(lsp);
        self.preview.clear();
        if self.picker.query().is_empty() {
            self.workspace_deadline = None;
            self.picker.set_workspace_symbols(Vec::new());
            return;
        }
        self.workspace_deadline = Some(now.checked_add(WORKSPACE_DEBOUNCE).unwrap_or(now));
        self.picker.set_loading(true);
    }

    pub(super) fn tick(&mut self, now: Instant, lsp: &mut LspUi) -> bool {
        let mut changed = false;
        if self
            .workspace_deadline
            .is_some_and(|deadline| now >= deadline)
        {
            self.workspace_deadline = None;
            changed |= self.submit_workspace(lsp);
        }
        changed |= self.preview.tick(now);
        changed |= self.preview.poll();
        changed
    }

    fn submit_workspace(&mut self, lsp: &mut LspUi) -> bool {
        let query = self.picker.query().to_string();
        if query.is_empty() {
            return false;
        }
        match lsp.workspace_symbols(&query) {
            Ok(request) => {
                self.workspace_request = Some(request);
                self.picker.set_loading(true);
            }
            Err(message) => self.picker.set_status(Some(message)),
        }
        true
    }

    pub(super) fn wait_timeout(&self, now: Instant) -> Option<Duration> {
        [self.workspace_deadline, self.preview.deadline()]
            .into_iter()
            .flatten()
            .map(|deadline| deadline.saturating_duration_since(now))
            .min()
    }

    pub(crate) fn preview_title(&self) -> Option<&str> {
        self.preview
            .title()
            .or_else(|| self.preview.is_loading().then_some(PREVIEW_LOADING))
    }

    pub(crate) fn preview_lines(&self) -> &[String] {
        self.preview.lines()
    }

    pub(crate) fn preview_first_line(&self) -> usize {
        self.preview.first_line()
    }

    pub(crate) fn preview_total_lines(&self) -> usize {
        self.preview.total_lines()
    }

    pub(crate) fn preview_focus_line(&self) -> Option<usize> {
        self.preview.focus_line()
    }

    pub(crate) fn preview_syntax(&self) -> &[kawi_syntax::SyntaxSpan] {
        self.preview.syntax()
    }

    pub(crate) fn paste(
        &mut self,
        text: &str,
        editor: &Editor,
        lsp: &mut LspUi,
    ) -> Result<(), String> {
        self.picker.insert_text(text);
        if self.mode == SymbolMode::Workspace {
            self.query_changed(lsp, Instant::now());
        } else {
            self.schedule_preview(editor, lsp, Instant::now());
        }
        Ok(())
    }

    pub(super) fn close(&mut self, lsp: &mut LspUi) {
        self.cancel_workspace_request(lsp);
        self.cancel_document_request(lsp);
        self.cancel_resolve(lsp);
        self.preview.clear();
    }

    fn cancel_workspace_request(&mut self, lsp: &mut LspUi) {
        if let Some(request) = self.workspace_request.take() {
            lsp.cancel_symbol_request(request);
        }
        self.workspace_deadline = None;
    }

    fn cancel_document_request(&mut self, lsp: &mut LspUi) {
        if let Some(request) = self.document_request.take() {
            lsp.cancel_symbol_request(request);
        }
    }

    pub(super) fn selected_row(&self) -> Option<&PickerRow> {
        let selected = self.picker.selected()?;
        self.picker.rows().get(selected)
    }

    pub(super) fn selected_identity(&self) -> Option<Identity> {
        let id = self.selected_row()?.id?;
        self.entries.get(&id).map(identity)
    }
}
