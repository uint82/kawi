use crate::client::{Client, ClientEvent, ClientKey};
use crate::protocol::RequestId;
use crate::Languages;
use anyhow::{Context, Result};
use lsp_types::{PositionEncodingKind, TextDocumentSyncKind, TextDocumentSyncSaveOptions};
use ropey::Rope;
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

mod code_actions;
mod commands;
mod completion;
mod completion_candidates;
mod completion_edit;
mod completion_entry;
mod completion_resolve;
mod completion_snippet;
mod diagnostics;
mod document;
mod file_operations;
mod formatting;
mod initialize;
mod manager;
mod markup;
mod navigation;
mod navigation_results;
mod pending;
mod pull_diagnostics;
mod rename;
mod request;
mod signature_help;
mod symbols;
mod text_requests;
mod watched_files;
mod workspace_diagnostics;
mod workspace_edit;

const INITIALIZE_TIMEOUT: Duration = Duration::from_secs(12);
pub const INITIAL_DOCUMENT_VERSION: i32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticMode {
    Push,
    Pull,
    Workspace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticModeEvent {
    pub server: String,
    pub mode: DiagnosticMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceDiagnosticState {
    Started,
    Finished,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceDiagnosticEvent {
    pub server: String,
    pub state: WorkspaceDiagnosticState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchedFileChange {
    Created,
    Changed,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerState {
    Starting,
    Initializing,
    Ready,
    Missing,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusEvent {
    pub server: String,
    pub state: ServerState,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressEvent {
    pub server: String,
    pub token: String,
    pub title: Option<String>,
    pub message: Option<String>,
    pub percentage: Option<u32>,
    pub done: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageLevel {
    Error,
    Warning,
    Info,
    Log,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPosition {
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextLocation {
    pub path: PathBuf,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationEvent {
    pub request: u64,
    pub path: PathBuf,
    pub position: CursorPosition,
    pub locations: Vec<TextLocation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkupDocument {
    pub text: String,
    pub markdown: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverEvent {
    pub request: u64,
    pub path: PathBuf,
    pub position: CursorPosition,
    pub contents: Vec<MarkupDocument>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionTriggerKind {
    Invoked,
    TriggerCharacter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionTrigger {
    pub kind: CompletionTriggerKind,
    pub character: Option<String>,
}

impl CompletionTrigger {
    pub fn invoked() -> Self {
        Self {
            kind: CompletionTriggerKind::Invoked,
            character: None,
        }
    }

    pub fn character(character: impl Into<String>) -> Self {
        Self {
            kind: CompletionTriggerKind::TriggerCharacter,
            character: Some(character.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentEdit {
    pub path: PathBuf,
    pub original_hash: u64,
    pub edits: Vec<TextChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkspaceEdit {
    pub documents: Vec<DocumentEdit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameReady {
    pub request: u64,
    pub placeholder: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameEvent {
    pub request: u64,
    pub edit: WorkspaceEdit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOperationKind {
    Rename,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileOperationEvent {
    pub request: u64,
    pub kind: FileOperationKind,
    pub edit: Option<WorkspaceEdit>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileOperationCapabilitiesEvent {
    pub server: String,
    pub rename: bool,
    pub delete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspCommand {
    pub title: String,
    pub command: String,
    pub arguments: Option<Vec<Value>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionEntry {
    pub server: String,
    pub label: String,
    pub filter_text: Option<String>,
    pub sort_text: Option<String>,
    pub detail: Option<String>,
    pub documentation: Option<MarkupDocument>,
    pub primary: TextChange,
    pub additional: Vec<TextChange>,
    pub deprecated: bool,
    pub preselect: bool,
    pub snippet: bool,
    pub resolve_id: Option<u64>,
    pub command: Option<LspCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionEvent {
    pub request: u64,
    pub server: String,
    pub path: PathBuf,
    pub version: i32,
    pub original_hash: u64,
    pub cursor: usize,
    pub word_start: usize,
    pub prefix: String,
    pub items: Vec<CompletionEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionResolveEvent {
    pub request: u64,
    pub item_id: u64,
    pub original_hash: u64,
    pub detail: Option<String>,
    pub documentation: Option<MarkupDocument>,
    pub additional: Vec<TextChange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureParameter {
    pub label: String,
    pub documentation: Option<MarkupDocument>,
    pub range: Option<(usize, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureEntry {
    pub label: String,
    pub documentation: Option<MarkupDocument>,
    pub parameters: Vec<SignatureParameter>,
    pub active_parameter: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelpEvent {
    pub request: u64,
    pub server: String,
    pub signatures: Vec<SignatureEntry>,
    pub active_signature: Option<usize>,
    pub active_parameter: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionCapabilitiesEvent {
    pub server: String,
    pub completion: bool,
    pub completion_triggers: Vec<String>,
    pub completion_resolve: bool,
    pub signature_help: bool,
    pub signature_triggers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolCapabilitiesEvent {
    pub server: String,
    pub document: bool,
    pub workspace: bool,
    pub resolve: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeActionEntry {
    pub server: String,
    pub title: String,
    pub kind: Option<String>,
    pub preferred: bool,
    pub edit: Option<WorkspaceEdit>,
    pub command: Option<LspCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeActionEvent {
    pub request: u64,
    pub path: PathBuf,
    pub actions: Vec<CodeActionEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormattingEvent {
    pub request: u64,
    pub edit: WorkspaceEdit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormattingSettings {
    pub tab_size: u32,
    pub insert_spaces: bool,
}

pub use kawi_buffer::TextEdit as TextChange;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LspEvent {
    Status(StatusEvent),
    Progress(ProgressEvent),
    DiagnosticMode(DiagnosticModeEvent),
    WorkspaceDiagnostics(WorkspaceDiagnosticEvent),
    Diagnostics(DiagnosticsEvent),
    Definition(LocationEvent),
    References(LocationEvent),
    Hover(HoverEvent),
    RenameReady(RenameReady),
    Rename(RenameEvent),
    FileOperationCapabilities(FileOperationCapabilitiesEvent),
    FileOperation(FileOperationEvent),
    Completion(CompletionEvent),
    CompletionResolved(CompletionResolveEvent),
    CompletionCapabilities(CompletionCapabilitiesEvent),
    SymbolCapabilities(SymbolCapabilitiesEvent),
    SignatureHelp(SignatureHelpEvent),
    CodeActions(CodeActionEvent),
    Formatting(FormattingEvent),
    DocumentSymbols(DocumentSymbolEvent),
    WorkspaceSymbols(WorkspaceSymbolEvent),
    WorkspaceSymbolResolved(WorkspaceSymbolResolvedEvent),
    RequestError {
        request: u64,
        message: String,
    },
    Message {
        server: String,
        level: MessageLevel,
        message: String,
    },
    Log {
        server: String,
        message: String,
    },
}

pub use symbols::{
    DocumentSymbolEvent, SymbolEntry, SymbolKind, SymbolRange, WorkspaceSymbolEvent,
    WorkspaceSymbolResolvedEvent,
};

pub use diagnostics::{
    Diagnostic, DiagnosticPosition, DiagnosticRange, DiagnosticRelatedInformation,
    DiagnosticSeverity, DiagnosticTag, DiagnosticsEvent,
};

pub struct LspService {
    commands: Sender<ServiceInput>,
    events: Receiver<LspEvent>,
    next_request: AtomicU64,
    worker: Option<JoinHandle<()>>,
}

impl LspService {
    pub fn new(languages: Languages, wake: Sender<()>) -> Result<Self> {
        Self::with_initialize_timeout(languages, wake, INITIALIZE_TIMEOUT)
    }

    pub fn with_initialize_timeout(
        languages: Languages,
        wake: Sender<()>,
        initialize_timeout: Duration,
    ) -> Result<Self> {
        let (commands, input) = mpsc::channel();
        let (event_tx, events) = mpsc::channel();
        let manager_commands = commands.clone();
        let worker = thread::Builder::new()
            .name("kawi-lsp".to_string())
            .spawn(move || {
                Manager::new(
                    languages,
                    input,
                    manager_commands,
                    event_tx,
                    wake,
                    initialize_timeout,
                )
                .run()
            })
            .context("spawn lsp manager")?;
        Ok(Self {
            commands,
            events,
            next_request: AtomicU64::new(0),
            worker: Some(worker),
        })
    }

    pub fn open(&self, path: &Path, text: impl Into<Rope>) -> bool {
        self.send(Command::Open {
            path: path.to_path_buf(),
            text: text.into(),
        })
    }

    pub fn change(&self, path: &Path, text: impl Into<Rope>) -> bool {
        self.send(Command::Change {
            path: path.to_path_buf(),
            text: text.into(),
        })
    }

    pub fn save(&self, path: &Path) -> bool {
        self.send(Command::Save {
            path: path.to_path_buf(),
        })
    }

    pub fn close(&self, path: &Path) -> bool {
        self.send(Command::Close {
            path: path.to_path_buf(),
        })
    }

    pub fn definition(&self, path: &Path, position: CursorPosition) -> u64 {
        let request = self.allocate_request();
        self.send(Command::Definition {
            request,
            path: path.to_path_buf(),
            position,
        });
        request
    }

    pub fn references(&self, path: &Path, position: CursorPosition) -> u64 {
        let request = self.allocate_request();
        self.send(Command::References {
            request,
            path: path.to_path_buf(),
            position,
        });
        request
    }

    pub fn hover(&self, path: &Path, position: CursorPosition) -> u64 {
        let request = self.allocate_request();
        self.send(Command::Hover {
            request,
            path: path.to_path_buf(),
            position,
        });
        request
    }

    pub fn prepare_rename(&self, path: &Path, position: CursorPosition) -> u64 {
        let request = self.allocate_request();
        self.send(Command::PrepareRename {
            request,
            path: path.to_path_buf(),
            position,
        });
        request
    }

    pub fn rename(&self, request: u64, new_name: &str) -> bool {
        self.send(Command::Rename {
            request,
            new_name: new_name.to_string(),
        })
    }

    pub fn will_rename_files(&self, files: &[(PathBuf, PathBuf)]) -> u64 {
        let request = self.allocate_request();
        self.send(Command::WillRenameFiles {
            request,
            files: files.to_vec(),
        });
        request
    }

    pub fn did_rename_files(&self, files: &[(PathBuf, PathBuf)]) -> bool {
        self.send(Command::DidRenameFiles {
            files: files.to_vec(),
        })
    }

    pub fn will_delete_files(&self, files: &[PathBuf]) -> u64 {
        let request = self.allocate_request();
        self.send(Command::WillDeleteFiles {
            request,
            files: files.to_vec(),
        });
        request
    }

    pub fn did_delete_files(&self, files: &[PathBuf]) -> bool {
        self.send(Command::DidDeleteFiles {
            files: files.to_vec(),
        })
    }

    pub fn completion(
        &self,
        path: &Path,
        position: CursorPosition,
        trigger: CompletionTrigger,
    ) -> u64 {
        let request = self.allocate_request();
        self.send(Command::Completion {
            request,
            path: path.to_path_buf(),
            position,
            trigger,
        });
        request
    }

    pub fn resolve_completion(&self, item_id: u64) -> u64 {
        let request = self.allocate_request();
        self.send(Command::ResolveCompletion { request, item_id });
        request
    }

    pub fn signature_help(
        &self,
        path: &Path,
        position: CursorPosition,
        trigger: CompletionTrigger,
    ) -> u64 {
        let request = self.allocate_request();
        self.send(Command::SignatureHelp {
            request,
            path: path.to_path_buf(),
            position,
            trigger,
        });
        request
    }

    pub fn code_actions(
        &self,
        path: &Path,
        range: (CursorPosition, CursorPosition),
        diagnostics: Vec<Diagnostic>,
    ) -> u64 {
        let request = self.allocate_request();
        self.send(Command::CodeActions {
            request,
            path: path.to_path_buf(),
            range,
            diagnostics,
        });
        request
    }

    pub fn formatting(
        &self,
        path: &Path,
        range: Option<(CursorPosition, CursorPosition)>,
        settings: FormattingSettings,
    ) -> u64 {
        let request = self.allocate_request();
        self.send(Command::Formatting {
            request,
            path: path.to_path_buf(),
            range,
            settings,
        });
        request
    }

    pub fn document_symbols(&self, path: &Path, version: u64) -> u64 {
        let request = self.allocate_request();
        self.send(Command::DocumentSymbols {
            request,
            path: path.to_path_buf(),
            version,
        });
        request
    }

    pub fn workspace_symbols(&self, query: impl Into<String>) -> u64 {
        let request = self.allocate_request();
        self.send(Command::WorkspaceSymbols {
            request,
            query: query.into(),
        });
        request
    }

    pub fn resolve_workspace_symbol(&self, symbol_id: u64) -> u64 {
        let request = self.allocate_request();
        self.send(Command::ResolveWorkspaceSymbol { request, symbol_id });
        request
    }

    pub fn workspace_diagnostics(&self) -> bool {
        self.send(Command::WorkspaceDiagnostics)
    }

    pub fn did_change_watched_files(&self, files: &[(PathBuf, WatchedFileChange)]) -> bool {
        self.send(Command::WatchedFiles {
            files: files.to_vec(),
        })
    }

    pub fn execute_command(&self, path: &Path, server: &str, command: &LspCommand) -> bool {
        self.send(Command::Execute {
            path: path.to_path_buf(),
            server: server.to_string(),
            command: command.command.clone(),
            arguments: command.arguments.clone(),
        })
    }

    pub fn cancel(&self, request: u64) {
        let _ = self.send(Command::Cancel { request });
    }

    pub fn try_recv(&self) -> Option<LspEvent> {
        self.events.try_recv().ok()
    }

    fn allocate_request(&self) -> u64 {
        self.next_request
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1)
    }

    fn send(&self, command: Command) -> bool {
        self.commands.send(ServiceInput::Command(command)).is_ok()
    }
}

impl Drop for LspService {
    fn drop(&mut self) {
        let _ = self.commands.send(ServiceInput::Command(Command::Shutdown));
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub(crate) enum InputWait {
    Input(ServiceInput),
    Timeout,
    Closed,
}

pub(crate) enum Command {
    Open {
        path: PathBuf,
        text: Rope,
    },
    Change {
        path: PathBuf,
        text: Rope,
    },
    Save {
        path: PathBuf,
    },
    Close {
        path: PathBuf,
    },
    Definition {
        request: u64,
        path: PathBuf,
        position: CursorPosition,
    },
    References {
        request: u64,
        path: PathBuf,
        position: CursorPosition,
    },
    Hover {
        request: u64,
        path: PathBuf,
        position: CursorPosition,
    },
    PrepareRename {
        request: u64,
        path: PathBuf,
        position: CursorPosition,
    },
    Rename {
        request: u64,
        new_name: String,
    },
    WillRenameFiles {
        request: u64,
        files: Vec<(PathBuf, PathBuf)>,
    },
    DidRenameFiles {
        files: Vec<(PathBuf, PathBuf)>,
    },
    WillDeleteFiles {
        request: u64,
        files: Vec<PathBuf>,
    },
    DidDeleteFiles {
        files: Vec<PathBuf>,
    },
    Completion {
        request: u64,
        path: PathBuf,
        position: CursorPosition,
        trigger: CompletionTrigger,
    },
    ResolveCompletion {
        request: u64,
        item_id: u64,
    },
    SignatureHelp {
        request: u64,
        path: PathBuf,
        position: CursorPosition,
        trigger: CompletionTrigger,
    },
    CodeActions {
        request: u64,
        path: PathBuf,
        range: (CursorPosition, CursorPosition),
        diagnostics: Vec<Diagnostic>,
    },
    Formatting {
        request: u64,
        path: PathBuf,
        range: Option<(CursorPosition, CursorPosition)>,
        settings: FormattingSettings,
    },
    DocumentSymbols {
        request: u64,
        path: PathBuf,
        version: u64,
    },
    WorkspaceSymbols {
        request: u64,
        query: String,
    },
    ResolveWorkspaceSymbol {
        request: u64,
        symbol_id: u64,
    },
    Execute {
        path: PathBuf,
        server: String,
        command: String,
        arguments: Option<Vec<Value>>,
    },
    WorkspaceDiagnostics,
    WatchedFiles {
        files: Vec<(PathBuf, WatchedFileChange)>,
    },
    Cancel {
        request: u64,
    },
    Shutdown,
}

pub(crate) enum ServiceInput {
    Command(Command),
    Client(ClientEvent),
}

struct Manager {
    languages: Languages,
    input: Receiver<ServiceInput>,
    input_tx: Sender<ServiceInput>,
    events: Sender<LspEvent>,
    wake: Sender<()>,
    clients: HashMap<ClientKey, ClientState>,
    documents: HashMap<PathBuf, Document>,
    pending_requests: HashMap<u64, navigation::PendingRequest>,
    pending_replies: HashMap<RequestId, pending::PendingReply>,
    pending_renames: HashMap<u64, rename::PendingRename>,
    pending_file_operations: HashMap<u64, PendingFileOperation>,
    pending_text_requests: HashMap<u64, text_requests::PendingTextRequest>,
    pending_completion_resolves: HashMap<u64, completion_resolve::PendingResolve>,
    completion_resolve: HashMap<u64, completion_resolve::ResolveItem>,
    pending_symbols: HashMap<u64, symbols::PendingSymbols>,
    pending_symbol_resolves: HashMap<u64, symbols::PendingSymbolResolve>,
    symbol_resolve_items: HashMap<u64, symbols::StoredWorkspaceSymbol>,
    next_symbol_item: u64,
    pending_commands: HashMap<RequestId, ClientKey>,
    pull_diagnostics: pull_diagnostics::PullDiagnostics,
    workspace_diagnostics: workspace_diagnostics::WorkspaceDiagnostics,
    next_request: i64,
    next_completion_item: u64,
    initialize_timeout: Duration,
}

struct ClientState {
    client: Client,
    status: ServerState,
    sync: SyncConfig,
    encoding: PositionEncodingKind,
    diagnostic_identifier: Option<String>,
    initialization_options: Option<Value>,
    settings: Value,
    pending_initialize: Option<RequestId>,
    initialize_deadline: Option<Instant>,
    opened: HashSet<PathBuf>,
    support: RequestSupport,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct RequestSupport {
    definition: bool,
    references: bool,
    hover: bool,
    rename: bool,
    prepare_rename: bool,
    completion: bool,
    completion_resolve: bool,
    signature_help: bool,
    code_actions: bool,
    formatting: bool,
    range_formatting: bool,
    pull_diagnostics: bool,
    workspace_diagnostics: bool,
    will_rename_files: bool,
    did_rename_files: bool,
    will_delete_files: bool,
    did_delete_files: bool,
    document_symbols: bool,
    workspace_symbols: bool,
    workspace_symbol_resolve: bool,
}

struct Document {
    uri: lsp_types::Uri,
    language_id: String,
    text: Rope,
    version: i32,
    clients: Vec<ClientKey>,
}

struct PendingFileOperation {
    kind: FileOperationKind,
    remaining: HashSet<ClientKey>,
    edit: Option<WorkspaceEdit>,
    deadline: Instant,
}

#[derive(Default)]
struct SyncConfig {
    open_close: bool,
    change: Option<TextDocumentSyncKind>,
    save: Option<TextDocumentSyncSaveOptions>,
}

pub(super) fn to_params<T: Serialize>(params: T) -> Option<Value> {
    serde_json::to_value(params).ok()
}

pub(super) fn one_line_message(message: &str) -> String {
    let sanitized: String = message
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect();
    sanitized.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn convert_command(command: lsp_types::Command) -> LspCommand {
    LspCommand {
        title: one_line_message(&command.title),
        command: command.command,
        arguments: command.arguments,
    }
}
