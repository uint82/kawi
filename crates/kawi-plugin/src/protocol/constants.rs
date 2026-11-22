pub const PROTOCOL_VERSION: u64 = 1;
pub const PROTOCOL_MINOR: u64 = 19;
pub const PROCESS_PROTOCOL_MINOR: u64 = 12;
pub const POSITION_ENCODING: &str = "utf-8-bytes";
pub const HOST_METHODS: &[&str] = crate::api_maturity::HOST_METHOD_NAMES;
pub const PLUGIN_METHODS: [&str; 1] = ["invoke_command"];
pub const INTERACTIVE_HOST_METHODS: [&str; 5] = [
    "ui.input",
    "ui.select",
    "ui.confirm",
    "ui.overlay_wait",
    "ui.picker",
];
pub const MAX_PLUGIN_DECORATIONS: usize = 10_000;
pub const MAX_DECORATION_TEXT_BYTES: usize = 512;
pub const MAX_PLUGIN_STATUSLINE_SEGMENTS: usize = 16;
pub const MAX_STATUSLINE_TEXT_BYTES: usize = 256;
pub const MAX_PLUGIN_DIAGNOSTICS: usize = 1_000;
pub const MAX_DIAGNOSTIC_MESSAGE_BYTES: usize = 4_096;
pub const MAX_DIAGNOSTIC_CODE_BYTES: usize = 256;
pub const MAX_PLUGIN_TIMERS: usize = 64;
pub const MIN_TIMER_INTERVAL_MS: u32 = 10;
pub const MAX_TIMER_DELAY_MS: u32 = 24 * 60 * 60 * 1_000;
pub const MAX_TIMER_PAYLOAD_BYTES: usize = 4_096;
pub const MAX_VARIABLE_KEY_BYTES: usize = 128;
pub const MAX_VARIABLE_VALUE_BYTES: usize = 4_096;
pub const MAX_GLOBAL_VARIABLES: usize = 1_024;
pub const MAX_PLUGIN_VARIABLES: usize = 256;
pub const MAX_BUFFER_VARIABLES: usize = 256;
pub const MAX_WINDOW_VARIABLES: usize = 256;

pub const PARSE_ERROR: i64 = -32700;
pub const INVALID_REQUEST: i64 = -32600;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;
pub const INTERNAL_ERROR: i64 = -32603;
pub const PLUGIN_ERROR: i64 = -32000;
pub const STALE_REVISION: i64 = -32010;
pub const INVALID_POSITION: i64 = -32011;
pub const CAPABILITY_DENIED: i64 = -32012;
pub const REQUEST_CANCELED: i64 = -32020;
pub const PLUGIN_UNAVAILABLE: i64 = -32030;
pub const UI_BUSY: i64 = -32040;
pub const JOB_NOT_FOUND: i64 = -32050;
pub const JOB_LIMIT: i64 = -32051;
pub const JOB_CLOSED: i64 = -32052;
pub const JOB_QUEUE_FULL: i64 = -32053;
pub const PANEL_NOT_FOUND: i64 = -32060;
pub const OVERLAY_NOT_FOUND: i64 = -32061;
pub const TASK_NOT_FOUND: i64 = -32062;
pub const TASK_LIMIT: i64 = -32063;
pub const TASK_START_FAILED: i64 = -32064;
pub const BUFFER_NOT_FOUND: i64 = -32070;
pub const BUFFER_NOT_ACTIVE: i64 = -32071;
pub const WINDOW_NOT_FOUND: i64 = -32072;
pub const LAST_WINDOW: i64 = -32073;
pub const OPTION_NOT_FOUND: i64 = -32074;
pub const MODE_NOT_SUPPORTED: i64 = -32075;
pub const BUFFER_MODIFIED: i64 = -32076;
pub const BUFFER_NOT_OWNED: i64 = -32077;
