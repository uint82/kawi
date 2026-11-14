pub const PROTOCOL_VERSION: u64 = 1;
pub const PROTOCOL_MINOR: u64 = 13;
pub const PROCESS_PROTOCOL_MINOR: u64 = 12;
pub const POSITION_ENCODING: &str = "utf-8-bytes";
pub const HOST_METHODS: [&str; 41] = [
    "editor.get_state",
    "editor.apply_edit",
    "editor.execute",
    "editor.list_commands",
    "editor.set_diagnostics",
    "editor.set_decorations",
    "ui.set_statusline",
    "ui.message",
    "ui.input",
    "ui.select",
    "ui.confirm",
    "ui.progress_start",
    "ui.progress_update",
    "ui.progress_finish",
    "ui.panel_open",
    "ui.panel_update",
    "ui.panel_focus",
    "ui.panel_close",
    "config.get",
    "config.list",
    "process.open",
    "process.write",
    "process.close_stdin",
    "process.signal",
    "process.kill",
    "process.status",
    "process.poll",
    "buffer.list",
    "buffer.get",
    "buffer.open",
    "buffer.apply_edit",
    "window.list",
    "window.get",
    "window.focus",
    "window.open",
    "window.close",
    "editor.set_cursor",
    "editor.set_selection",
    "editor.set_mode",
    "options.list",
    "options.get",
];
pub const PLUGIN_METHODS: [&str; 1] = ["invoke_command"];
pub const INTERACTIVE_HOST_METHODS: [&str; 3] = ["ui.input", "ui.select", "ui.confirm"];
pub const MAX_PLUGIN_DECORATIONS: usize = 10_000;
pub const MAX_DECORATION_TEXT_BYTES: usize = 512;
pub const MAX_PLUGIN_STATUSLINE_SEGMENTS: usize = 16;
pub const MAX_STATUSLINE_TEXT_BYTES: usize = 256;
pub const MAX_PLUGIN_DIAGNOSTICS: usize = 1_000;
pub const MAX_DIAGNOSTIC_MESSAGE_BYTES: usize = 4_096;
pub const MAX_DIAGNOSTIC_CODE_BYTES: usize = 256;

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
pub const BUFFER_NOT_FOUND: i64 = -32070;
pub const BUFFER_NOT_ACTIVE: i64 = -32071;
pub const WINDOW_NOT_FOUND: i64 = -32072;
pub const LAST_WINDOW: i64 = -32073;
pub const OPTION_NOT_FOUND: i64 = -32074;
pub const MODE_NOT_SUPPORTED: i64 = -32075;
