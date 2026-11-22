mod api_maturity;
pub mod api_metadata;
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
    import_capabilities, import_rule, requested_import_capabilities, set_wasm_cache_dir,
    unauthorized_imports, ComponentInspection, ImportRule, WasiConfig, WasmRuntime, WASM_IMPORTS,
};
#[cfg(all(feature = "wasm-test-network", debug_assertions))]
pub use wasm::{install_resolver_script, remove_resolver_script, resolver_call_count};

pub use api_maturity::{
    event_maturity, event_since, host_method_maturity, host_method_since, ApiMaturity,
    ApiStability, MaturityError,
};
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
    decoration_ref, event_value, plugin_origin, sanitize, validate_autocmd_pattern,
    validate_autocmd_spec, validate_command_id, validate_command_spec, validate_decoration_scope,
    validate_decorations, validate_diagnostics, validate_keymap_keys, validate_keymap_spec,
    validate_option_value, validate_panel_open, validate_panel_update, validate_statusline,
    validate_timer_count, validate_timer_delay, validate_timer_interval, validate_timer_payload,
    validate_user_event, validate_variable_key, validate_variable_value, ApplyEditParams,
    ApplyEditResult, AutocmdRemoveParams, AutocmdSpec, BufferChangedEvent, BufferCloseParams,
    BufferClosedEvent, BufferCreateParams, BufferEnterEvent, BufferGetParams, BufferInfo,
    BufferLeaveEvent, BufferOpenParams, BufferOpenedEvent, BufferState, CommandRegisterParams,
    CommandUnregisterParams, ConfigGetParams, ConfigGetResult, ConfigListResult, ConfigSetting,
    CursorMovedEvent, Decoration, DecorationFill, DecorationRef, DecorationScope, DecorationStyle,
    DiagnosticSummary, DiagnosticsChangedEvent, EditorCommandInfo, EditorExecuteParams,
    EditorListCommandsResult, FileOpenedEvent, FileSavedEvent, FocusChangedEvent, GetStateParams,
    GetStateResult, InitializeParams, InitializeResult, InvocationContext, InvokeParams,
    JobExitedEvent, JobOutputEvent, JobState, JobStream, KeyMode, KeymapRemoveParams,
    KeymapSetParams, LanguageChangedEvent, ModeChangedEvent, OptionInfo, OptionsGetParams,
    OptionsListParams, OptionsResetParams, OptionsSetParams, PanelLine, PanelOpenParams,
    PanelOpenResult, PanelParams, PanelUpdateParams, Position, ProcessJobEvent, ProcessJobParams,
    ProcessOpenParams, ProcessOpenResult, ProcessSignal, ProcessSignalParams, ProcessStatusResult,
    ProcessWriteParams, ProtocolTextEdit, RegisteredAutocmd, RegisteredCommand, RegisteredKeymap,
    RpcError, Selection, SelectionChangedEvent, SetCursorParams, SetDecorationsParams,
    SetDiagnosticsParams, SetModeParams, SetSelectionParams, SetStatuslineParams,
    StatuslineSegment, StdinMode, TimerAfterParams, TimerCancelParams, TimerEveryParams,
    TimerFired, TimerInfo, UiConfirmParams, UiInputParams, UiMessageParams, UiProgressFinishParams,
    UiProgressOpenResult, UiProgressStartParams, UiProgressUpdateParams, UiProgressUpdateResult,
    UiPromptOutcome, UiSelectParams, UserEvent, UserEventParams, Variable, VariableParams,
    VariableScope, VariableSetParams, VariableTarget, VariableTargetParams, WindowCloseParams,
    WindowClosedEvent, WindowCreatedEvent, WindowEnterEvent, WindowInfo, WindowLeaveEvent,
    WindowOpenParams, WindowParams, BUFFER_MODIFIED, BUFFER_NOT_ACTIVE, BUFFER_NOT_FOUND,
    BUFFER_NOT_OWNED, CAPABILITY_DENIED, DEFAULT_PLUGIN_PANEL_HEIGHT, EVENT_NAMES, HOST_METHODS,
    INTERNAL_ERROR, INVALID_PARAMS, INVALID_POSITION, INVALID_REQUEST, JOB_CLOSED,
    JOB_EXITED_EVENT, JOB_LIMIT, JOB_NOT_FOUND, JOB_OUTPUT_EVENT, JOB_QUEUE_FULL, LAST_WINDOW,
    MAX_AUTOCMD_PATTERN_BYTES, MAX_BUFFER_VARIABLES, MAX_DECORATION_TEXT_BYTES,
    MAX_DIAGNOSTIC_CODE_BYTES, MAX_DIAGNOSTIC_MESSAGE_BYTES, MAX_GLOBAL_VARIABLES,
    MAX_JOB_EVENT_QUEUE_BYTES, MAX_JOB_EVENT_QUEUE_ITEMS, MAX_PANEL_LINE_BYTES,
    MAX_PANEL_STATUS_BYTES, MAX_PANEL_TITLE_BYTES, MAX_PLUGIN_AUTOCMDS, MAX_PLUGIN_DECORATIONS,
    MAX_PLUGIN_DIAGNOSTICS, MAX_PLUGIN_PANEL_BYTES, MAX_PLUGIN_PANEL_HEIGHT,
    MAX_PLUGIN_PANEL_LINES, MAX_PLUGIN_STATUSLINE_SEGMENTS, MAX_PLUGIN_TIMERS,
    MAX_PLUGIN_VARIABLES, MAX_RUNTIME_COMMANDS_PER_PLUGIN, MAX_RUNTIME_COMMAND_ALIASES,
    MAX_RUNTIME_COMMAND_DESCRIPTION_BYTES, MAX_RUNTIME_COMMAND_ID_BYTES,
    MAX_RUNTIME_COMMAND_TITLE_BYTES, MAX_RUNTIME_KEYMAPS_PER_PLUGIN,
    MAX_RUNTIME_KEYMAP_COMMAND_BYTES, MAX_RUNTIME_KEYMAP_DESCRIPTION_BYTES,
    MAX_RUNTIME_KEYMAP_KEYS_BYTES, MAX_STATUSLINE_TEXT_BYTES, MAX_TIMER_DELAY_MS,
    MAX_TIMER_PAYLOAD_BYTES, MAX_USER_EVENT_NAME_BYTES, MAX_USER_EVENT_PAYLOAD_BYTES,
    MAX_VARIABLE_KEY_BYTES, MAX_VARIABLE_VALUE_BYTES, MAX_WINDOW_VARIABLES, METHOD_NOT_FOUND,
    MIN_PLUGIN_PANEL_HEIGHT, MIN_TIMER_INTERVAL_MS, MODE_NOT_SUPPORTED, OPTION_NOT_FOUND,
    ORIGIN_EXTERNAL_RELOAD, ORIGIN_MIXED, ORIGIN_UNDO, ORIGIN_USER, PANEL_NOT_FOUND, PARSE_ERROR,
    PLUGIN_ERROR, PLUGIN_UNAVAILABLE, POSITION_ENCODING, PROCESS_PROTOCOL_MINOR, PROTOCOL_MINOR,
    PROTOCOL_VERSION, REQUEST_CANCELED, STALE_REVISION, UI_BUSY, WINDOW_NOT_FOUND,
};
pub use protocol::{
    validate_overlay_open, validate_overlay_update, validate_picker, Key, KeyModifier,
    OverlayAnchor, OverlayCloseReason, OverlayEvent, OverlayKeyEvent, OverlayLine,
    OverlayOpenParams, OverlayOpenResult, OverlayParams, OverlaySpan, OverlayUpdateParams,
    PickerItem, PickerOutcome, PickerParams, DEFAULT_OVERLAY_HEIGHT, DEFAULT_OVERLAY_WIDTH,
    MAX_OVERLAY_BYTES, MAX_OVERLAY_EVENTS, MAX_OVERLAY_HEIGHT, MAX_OVERLAY_LINES,
    MAX_OVERLAY_LINE_BYTES, MAX_OVERLAY_STATUS_BYTES, MAX_OVERLAY_TITLE_BYTES, MAX_OVERLAY_WIDTH,
    MAX_PICKER_DETAIL_BYTES, MAX_PICKER_ID_BYTES, MAX_PICKER_ITEMS, MAX_PICKER_LABEL_BYTES,
    MIN_OVERLAY_HEIGHT, MIN_OVERLAY_WIDTH, OVERLAY_NOT_FOUND,
};
pub use protocol::{
    TaskEvent, TaskExitedEvent, TaskFindingEvent, TaskInfo, TaskJobParams, TaskOutputEvent,
    TaskSeverity, TaskStartParams, TaskStartResult, TaskState, TaskStatusResult, TaskStream,
    MAX_TASKS_PER_PLUGIN, MAX_TASK_EVENT_QUEUE_BYTES, MAX_TASK_EVENT_QUEUE_ITEMS,
    MAX_TASK_FINDING_MESSAGE_BYTES, MAX_TASK_FINDING_PATH_BYTES, MAX_TASK_LINE_BYTES,
    MAX_TASK_NAME_BYTES, TASK_EXITED_EVENT, TASK_FINDING_EVENT, TASK_LIMIT, TASK_NOT_FOUND,
    TASK_OUTPUT_EVENT, TASK_RETENTION_SECS, TASK_START_FAILED,
};
