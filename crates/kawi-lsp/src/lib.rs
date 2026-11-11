mod client;
mod config;
mod defaults;
mod editing;
mod file;
pub use kawi_jsonrpc::framing;
mod process;
pub mod protocol;
mod service;
mod text;
mod warning;
mod workspace;

pub use config::{
    FormatterConfig, IndentConfig, LanguageConfig, Languages, ServerConfig, MAX_SHIFTWIDTH,
};
pub use editing::{BlockCommentConfig, PairConfig};
pub use lsp_types::CompletionItemKind;
pub use service::{
    CodeActionEntry, CodeActionEvent, CompletionCapabilitiesEvent, CompletionEntry,
    CompletionEvent, CompletionResolveEvent, CompletionTrigger, CompletionTriggerKind,
    CursorPosition, Diagnostic, DiagnosticMode, DiagnosticModeEvent, DiagnosticOrigin,
    DiagnosticPosition, DiagnosticRange, DiagnosticRelatedInformation, DiagnosticSeverity,
    DiagnosticTag, DiagnosticsEvent, DocumentColor, DocumentColorCapabilitiesEvent,
    DocumentColorsEvent, DocumentEdit, DocumentSymbolEvent, FileOperationCapabilitiesEvent,
    FileOperationEvent, FileOperationKind, FormattingEvent, FormattingSettings, HoverEvent,
    LocationEvent, LspCommand, LspEvent, LspService, MarkupDocument, MessageLevel, ProgressEvent,
    RenameEvent, RenameReady, ServerState, SignatureEntry, SignatureHelpEvent, SignatureParameter,
    SignatureTrigger, SignatureTriggerKind, StatusEvent, SymbolCapabilitiesEvent, SymbolEntry,
    SymbolKind, SymbolRange, TextChange, TextLocation, WatchedFileChange, WorkspaceDiagnosticEvent,
    WorkspaceDiagnosticState, WorkspaceEdit, WorkspaceSymbolEvent, WorkspaceSymbolResolvedEvent,
    INITIAL_DOCUMENT_VERSION,
};
pub use warning::LanguageWarning;
