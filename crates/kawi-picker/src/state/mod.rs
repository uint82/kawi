use crate::grep::GrepHit;
use crate::input::TextInput;
use crate::selection::selection_at;

pub(super) const MAX_ROWS: usize = 2_000;
pub(super) const PAGE_ROWS: usize = 10;

mod keys;
mod refresh;

use refresh::{buffer_candidate, file_candidate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerKind {
    Files,
    Grep,
    Buffers,
    Locations,
    Symbols,
    WorkspaceSymbols,
    GitBranches,
    GitLog,
    GitCommitFiles,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerRow {
    pub display: String,
    pub path: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub buffer: Option<usize>,
    pub secondary: Option<String>,
    pub deprecated: bool,
    pub id: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerOutcome {
    Consumed,
    QueryChanged,
    Open(PickerRow),
    Cancel,
}

pub struct Picker {
    kind: PickerKind,
    title: String,
    query: TextInput,
    files: Vec<Candidate>,
    hits: Vec<GrepHit>,
    buffers: Vec<Candidate>,
    locations: Vec<Candidate>,
    rows: Vec<PickerRow>,
    selected: Option<usize>,
    loading: bool,
    status: Option<String>,
}

struct Candidate {
    row: PickerRow,
    folded: String,
}

impl Picker {
    pub fn files() -> Self {
        Self::new(PickerKind::Files, "Files")
    }

    pub fn grep() -> Self {
        Self::new(PickerKind::Grep, "Live Grep")
    }

    pub fn buffers(buffers: Vec<(usize, String)>) -> Self {
        let mut picker = Self::new(PickerKind::Buffers, "Buffers");
        picker.buffers = buffers.into_iter().map(buffer_candidate).collect();
        picker.refresh();
        picker
    }

    pub fn locations(title: impl Into<String>, rows: Vec<PickerRow>) -> Self {
        let mut picker = Self::new(PickerKind::Locations, title);
        picker.locations = rows.into_iter().map(Candidate::new).collect();
        picker.refresh();
        picker
    }

    pub fn git_branches(title: impl Into<String>, rows: Vec<PickerRow>) -> Self {
        let mut picker = Self::new(PickerKind::GitBranches, title);
        picker.locations = rows.into_iter().map(Candidate::new).collect();
        picker.refresh();
        picker
    }

    pub fn git_log(title: impl Into<String>) -> Self {
        let mut picker = Self::new(PickerKind::GitLog, title);
        picker.loading = true;
        picker
    }

    pub fn symbols(title: impl Into<String>, rows: Vec<PickerRow>) -> Self {
        let mut picker = Self::new(PickerKind::Symbols, title);
        picker.locations = rows.into_iter().map(Candidate::new).collect();
        picker.refresh();
        picker
    }

    pub fn workspace_symbols(title: impl Into<String>) -> Self {
        Self::new(PickerKind::WorkspaceSymbols, title)
    }

    pub fn git_commit_files(title: impl Into<String>, rows: Vec<PickerRow>) -> Self {
        let mut picker = Self::new(PickerKind::GitCommitFiles, title);
        picker.locations = rows.into_iter().map(Candidate::new).collect();
        picker.refresh();
        picker
    }

    fn new(kind: PickerKind, title: impl Into<String>) -> Self {
        Self {
            kind,
            title: title.into(),
            query: TextInput::new(),
            files: Vec::new(),
            hits: Vec::new(),
            buffers: Vec::new(),
            locations: Vec::new(),
            rows: Vec::new(),
            selected: None,
            loading: false,
            status: None,
        }
    }

    pub fn kind(&self) -> PickerKind {
        self.kind
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn query(&self) -> &str {
        self.query.text()
    }

    pub fn cursor(&self) -> usize {
        self.query.cursor()
    }

    pub fn rows(&self) -> &[PickerRow] {
        &self.rows
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn loading(&self) -> bool {
        self.loading
    }

    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub fn needs_more_rows(&self) -> bool {
        self.rows.is_empty()
            || self.selected.unwrap_or_default().saturating_add(PAGE_ROWS) >= self.rows.len()
    }

    pub fn set_locations(&mut self, rows: Vec<PickerRow>) {
        self.locations = rows.into_iter().map(Candidate::new).collect();
        self.replace_rows(true);
    }

    pub fn set_workspace_symbols(&mut self, rows: Vec<PickerRow>) {
        self.locations = rows.into_iter().map(Candidate::new).collect();
        self.replace_rows(false);
        self.loading = false;
        self.status = None;
    }

    fn replace_rows(&mut self, refresh: bool) {
        let selected = self.selected_row_key();
        let selected_index = self.selected;
        self.rows = if refresh {
            self.location_rows()
        } else {
            self.locations
                .iter()
                .take(MAX_ROWS)
                .map(|candidate| candidate.row.clone())
                .collect()
        };
        self.selected = selected
            .and_then(|selected| self.rows.iter().position(|row| row.key() == selected))
            .or_else(|| selected_index.and_then(|index| selection_at(index, self.rows.len())))
            .or_else(|| (!self.rows.is_empty()).then_some(0));
    }

    fn selected_row_key(&self) -> Option<RowKey> {
        self.selected
            .and_then(|index| self.rows.get(index))
            .map(PickerRow::key)
    }

    pub fn select_symbol(&mut self, id: u64) -> bool {
        let Some(index) = self.rows.iter().position(|row| row.id == Some(id)) else {
            return false;
        };
        self.selected = Some(index);
        true
    }

    pub fn set_status(&mut self, status: Option<String>) {
        self.status = status;
        self.loading = false;
    }

    pub fn set_files(&mut self, files: Vec<String>) {
        self.files = files.into_iter().map(file_candidate).collect();
        self.loading = false;
        self.status = None;
        self.refresh();
    }

    pub fn set_hits(&mut self, hits: Vec<GrepHit>) {
        self.hits = hits;
        self.loading = false;
        self.status = None;
        self.refresh();
    }

    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }

    pub fn set_query(&mut self, query: &str) {
        self.query.set_text(query);
        self.after_query_change();
    }

    pub fn insert_text(&mut self, text: &str) {
        self.query.insert_text(text);
        self.after_query_change();
    }

    pub fn set_error(&mut self, error: String) {
        self.hits.clear();
        self.loading = false;
        self.status = Some(error);
        self.refresh();
    }

    pub fn clear_hits(&mut self) {
        self.hits.clear();
        self.rows.clear();
        self.selected = None;
        self.loading = false;
        self.status = None;
    }
}

type RowKey = (
    Option<u64>,
    String,
    Option<usize>,
    Option<usize>,
    Option<usize>,
);

impl PickerRow {
    fn key(&self) -> RowKey {
        (
            self.id,
            self.path.clone(),
            self.line,
            self.column,
            self.buffer,
        )
    }
}

#[cfg(test)]
mod tests;
