use super::*;
use ropey::Rope;
use serde_json::Value;

mod convert;
mod document;
mod kinds;
mod reply;
mod request;
mod workspace;

pub use kinds::SymbolKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolRange {
    pub start: CursorPosition,
    pub end: CursorPosition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolEntry {
    pub name: String,
    pub detail: Option<String>,
    pub container: Option<String>,
    pub kind: SymbolKind,
    pub deprecated: bool,
    pub path: PathBuf,
    pub range: Option<SymbolRange>,
    pub selection: Option<CursorPosition>,
    pub server: Option<String>,
    pub resolve_id: Option<u64>,
}

impl SymbolEntry {
    pub fn is_jumpable(&self) -> bool {
        self.selection.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSymbolEvent {
    pub request: u64,
    pub path: PathBuf,
    pub version: u64,
    pub symbols: Vec<SymbolEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSymbolEvent {
    pub request: u64,
    pub query: String,
    pub symbols: Vec<SymbolEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSymbolResolvedEvent {
    pub request: u64,
    pub symbol_id: u64,
    pub symbol: SymbolEntry,
}

#[derive(Debug, Clone)]
pub(super) struct PendingSymbols {
    request: u64,
    kind: PendingSymbolKind,
    remaining: usize,
    responses: Vec<SymbolResponse>,
    resolve_support: bool,
    error: Option<String>,
}

#[derive(Debug, Clone)]
enum PendingSymbolKind {
    Document {
        path: PathBuf,
        version: u64,
        text: Rope,
    },
    Workspace {
        query: String,
    },
}

#[derive(Debug, Clone)]
struct SymbolResponse {
    server: String,
    key: ClientKey,
    value: Value,
    encoding: PositionEncodingKind,
}

#[derive(Debug, Clone)]
pub(super) struct PendingSymbolResolve {
    symbol_id: u64,
}

#[derive(Debug, Clone)]
pub(super) struct StoredWorkspaceSymbol {
    key: ClientKey,
    symbol: lsp_types::WorkspaceSymbol,
    path: PathBuf,
}
