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

mod diagnostics;
mod document;
mod initialize;
mod manager;
mod navigation;
mod navigation_results;
mod request;

const INITIALIZE_TIMEOUT: Duration = Duration::from_secs(12);
pub const INITIAL_DOCUMENT_VERSION: i32 = 1;

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
pub struct HoverEvent {
    pub request: u64,
    pub path: PathBuf,
    pub position: CursorPosition,
    pub contents: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LspEvent {
    Status(StatusEvent),
    Progress(ProgressEvent),
    Diagnostics(DiagnosticsEvent),
    Definition(LocationEvent),
    References(LocationEvent),
    Hover(HoverEvent),
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

pub use diagnostics::{
    Diagnostic, DiagnosticPosition, DiagnosticRange, DiagnosticSeverity, DiagnosticsEvent,
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
    pending_replies: HashMap<RequestId, navigation::PendingReply>,
    next_request: i64,
    initialize_timeout: Duration,
}

struct ClientState {
    client: Client,
    status: ServerState,
    sync: SyncConfig,
    encoding: PositionEncodingKind,
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
}

struct Document {
    uri: lsp_types::Uri,
    language_id: String,
    text: Rope,
    version: i32,
    clients: Vec<ClientKey>,
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
