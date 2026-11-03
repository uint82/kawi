pub mod capability;
mod decoration_group;

pub const NETWORK_WARNING_PREFIX: &str = "network warning: ";

mod environment;
mod error;
mod host;
mod jobs;
mod lockfile;
mod manifest;
mod outcome;
mod payload_size;
mod process;
mod protocol;
mod wasm;

#[cfg(feature = "fuzz")]
pub use wasm::fuzz;
#[cfg(feature = "wasm")]
pub use wasm::{
    import_capabilities, import_rule, requested_import_capabilities, unauthorized_imports,
    ComponentInspection, ImportRule, WasiConfig, WasmRuntime, WASM_IMPORTS,
};
#[cfg(all(feature = "wasm-test-network", debug_assertions))]
pub use wasm::{install_resolver_script, remove_resolver_script, resolver_call_count};

pub use capability::{
    advertised_events, advertised_host_methods, event_capability, host_method_capability,
    is_event_allowed, is_host_method_allowed, Capability, CapabilityRegistry, CapabilityRequests,
    EffectiveCapabilities, MethodCapability, Scope,
};
pub use decoration_group::{
    validate_decoration_group, MAX_DECORATION_GROUPS, MAX_DECORATION_GROUP_LEN,
    RESERVED_DECORATION_GROUPS,
};
pub use environment::HostEnvironment;
pub use error::PluginError;
pub use host::{PluginHost, PluginHostEvent, PluginStatus};
pub use lockfile::{load_managed_catalog, LockedPlugin, PluginLock};
pub use manifest::{
    load_catalog, validate_component_path, validate_plugin_name, ActivationEvent, LoadedPlugin,
    PluginCatalog, PluginCommand, PluginDecorationGroup, PluginManifest, PluginRuntime,
    PluginSetting, PluginSettingKind, PluginWarning,
};
pub use outcome::{normalize_error, normalize_result, normalize_rpc, Outcome};
pub use process::NotifyOutcome;
pub use protocol::{
    decoration_ref, event_value, plugin_origin, sanitize, validate_decoration_scope,
    validate_decorations, validate_diagnostics, validate_statusline, ApplyEditParams,
    ApplyEditResult, BufferChangedEvent, BufferClosedEvent, BufferOpenedEvent, ConfigGetParams,
    ConfigGetResult, ConfigListResult, ConfigSetting, Decoration, DecorationFill, DecorationRef,
    DecorationScope, DecorationStyle, DiagnosticSummary, DiagnosticsChangedEvent,
    EditorCommandInfo, EditorExecuteParams, EditorListCommandsResult, FileOpenedEvent,
    FileSavedEvent, GetStateParams, GetStateResult, InitializeParams, InitializeResult,
    InvocationContext, InvokeParams, JobExitedEvent, JobOutputEvent, JobState, JobStream,
    ModeChangedEvent, Position, ProcessJobEvent, ProcessJobParams, ProcessOpenParams,
    ProcessOpenResult, ProcessSignal, ProcessSignalParams, ProcessStatusResult, ProcessWriteParams,
    ProtocolTextEdit, RpcError, Selection, SetDecorationsParams, SetDiagnosticsParams,
    SetStatuslineParams, StatuslineSegment, StdinMode, UiConfirmParams, UiInputParams,
    UiMessageParams, UiProgressFinishParams, UiProgressOpenResult, UiProgressStartParams,
    UiProgressUpdateParams, UiProgressUpdateResult, UiPromptOutcome, UiSelectParams,
    CAPABILITY_DENIED, EVENT_NAMES, HOST_METHODS, INTERNAL_ERROR, INVALID_PARAMS, INVALID_POSITION,
    INVALID_REQUEST, JOB_CLOSED, JOB_EXITED_EVENT, JOB_LIMIT, JOB_NOT_FOUND, JOB_OUTPUT_EVENT,
    JOB_QUEUE_FULL, MAX_DECORATION_TEXT_BYTES, MAX_DIAGNOSTIC_CODE_BYTES,
    MAX_DIAGNOSTIC_MESSAGE_BYTES, MAX_JOB_EVENT_QUEUE_BYTES, MAX_JOB_EVENT_QUEUE_ITEMS,
    MAX_PLUGIN_DECORATIONS, MAX_PLUGIN_DIAGNOSTICS, MAX_PLUGIN_STATUSLINE_SEGMENTS,
    MAX_STATUSLINE_TEXT_BYTES, METHOD_NOT_FOUND, ORIGIN_EXTERNAL_RELOAD, ORIGIN_MIXED, ORIGIN_UNDO,
    ORIGIN_USER, PARSE_ERROR, PLUGIN_ERROR, PLUGIN_UNAVAILABLE, POSITION_ENCODING, PROTOCOL_MINOR,
    PROTOCOL_VERSION, REQUEST_CANCELED, STALE_REVISION, UI_BUSY,
};
