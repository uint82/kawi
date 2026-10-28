pub mod capability;

mod error;
mod host;
mod manifest;
mod process;
mod protocol;

pub use error::PluginError;
pub use host::{PluginHost, PluginHostEvent, PluginStatus};
pub use manifest::{
    load_catalog, ActivationEvent, LoadedPlugin, PluginCatalog, PluginCommand, PluginManifest,
    PluginWarning,
};
pub use process::NotifyOutcome;
pub use protocol::{
    event_value, plugin_origin, ApplyEditParams, ApplyEditResult, BufferChangedEvent,
    BufferClosedEvent, BufferOpenedEvent, DiagnosticSummary, DiagnosticsChangedEvent,
    FileOpenedEvent, FileSavedEvent, GetStateParams, GetStateResult, InitializeParams,
    InitializeResult, InvocationContext, InvokeParams, ModeChangedEvent, Position,
    ProtocolTextEdit, RpcError, Selection, UiMessageParams, EVENT_NAMES, HOST_METHODS,
    INTERNAL_ERROR, INVALID_PARAMS, INVALID_POSITION, INVALID_REQUEST, METHOD_NOT_FOUND,
    ORIGIN_EXTERNAL_RELOAD, ORIGIN_MIXED, ORIGIN_UNDO, ORIGIN_USER, PARSE_ERROR, PLUGIN_ERROR,
    PLUGIN_UNAVAILABLE, POSITION_ENCODING, PROTOCOL_MINOR, PROTOCOL_VERSION, REQUEST_CANCELED,
    STALE_REVISION,
};
