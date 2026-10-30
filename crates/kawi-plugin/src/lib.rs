pub mod capability;

pub const NETWORK_WARNING_PREFIX: &str = "network warning: ";

mod environment;
mod error;
mod host;
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
    is_event_allowed, is_host_method_allowed, load_permission_state, Capability,
    CapabilityRegistry, EffectiveCapabilities, MethodCapability, PermissionState, Scope,
};
pub use environment::HostEnvironment;
pub use error::PluginError;
pub use host::{PluginHost, PluginHostEvent, PluginStatus};
pub use manifest::{
    load_catalog, ActivationEvent, LoadedPlugin, PluginCatalog, PluginCommand, PluginManifest,
    PluginRuntime, PluginWarning,
};
pub use outcome::{normalize_error, normalize_result, normalize_rpc, Outcome};
pub use process::NotifyOutcome;
pub use protocol::{
    event_value, plugin_origin, sanitize, validate_diagnostics, ApplyEditParams, ApplyEditResult,
    BufferChangedEvent, BufferClosedEvent, BufferOpenedEvent, DiagnosticSummary,
    DiagnosticsChangedEvent, FileOpenedEvent, FileSavedEvent, GetStateParams, GetStateResult,
    InitializeParams, InitializeResult, InvocationContext, InvokeParams, ModeChangedEvent,
    Position, ProtocolTextEdit, RpcError, Selection, SetDiagnosticsParams, UiMessageParams,
    CAPABILITY_DENIED, EVENT_NAMES, HOST_METHODS, INTERNAL_ERROR, INVALID_PARAMS, INVALID_POSITION,
    INVALID_REQUEST, MAX_DIAGNOSTIC_CODE_BYTES, MAX_DIAGNOSTIC_MESSAGE_BYTES,
    MAX_PLUGIN_DIAGNOSTICS, METHOD_NOT_FOUND, ORIGIN_EXTERNAL_RELOAD, ORIGIN_MIXED, ORIGIN_UNDO,
    ORIGIN_USER, PARSE_ERROR, PLUGIN_ERROR, PLUGIN_UNAVAILABLE, POSITION_ENCODING, PROTOCOL_MINOR,
    PROTOCOL_VERSION, REQUEST_CANCELED, STALE_REVISION,
};
