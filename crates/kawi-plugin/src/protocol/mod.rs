use std::fmt;

mod diagnostics;
mod events;
mod sanitize;
mod types;

pub use events::{
    event_value, plugin_origin, BufferChangedEvent, BufferClosedEvent, BufferOpenedEvent,
    DiagnosticSummary, DiagnosticsChangedEvent, FileOpenedEvent, FileSavedEvent, ModeChangedEvent,
    EVENT_NAMES, ORIGIN_EXTERNAL_RELOAD, ORIGIN_MIXED, ORIGIN_UNDO, ORIGIN_USER,
};

use serde_json::Value;

pub use diagnostics::validate_diagnostics;
pub use sanitize::sanitize;

pub const PROTOCOL_VERSION: u64 = 1;
pub const PROTOCOL_MINOR: u64 = 5;
pub const POSITION_ENCODING: &str = "utf-8-bytes";
pub const HOST_METHODS: [&str; 14] = [
    "editor.get_state",
    "editor.apply_edit",
    "editor.execute",
    "editor.list_commands",
    "editor.set_diagnostics",
    "ui.message",
    "ui.input",
    "ui.select",
    "ui.confirm",
    "ui.progress_start",
    "ui.progress_update",
    "ui.progress_finish",
    "config.get",
    "config.list",
];
pub const PLUGIN_METHODS: [&str; 1] = ["invoke_command"];
pub const INTERACTIVE_HOST_METHODS: [&str; 3] = ["ui.input", "ui.select", "ui.confirm"];
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

pub use types::*;

pub fn is_interactive_host_method(method: &str) -> bool {
    INTERACTIVE_HOST_METHODS.contains(&method)
}

pub fn request_value(id: Value, method: &str, params: Value) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    })
}

pub fn response_value(id: Value, result: Value) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

pub fn error_response_value(id: Value, error: &RpcError) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": error.to_value(),
    })
}

pub fn notification_value(method: &str, params: Value) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

impl RpcError {
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(INVALID_PARAMS, message)
    }

    pub fn method_not_found(method: &str) -> Self {
        Self::new(METHOD_NOT_FOUND, format!("method not found: {method}"))
    }

    pub fn stale_revision(expected: u64, actual: u64) -> Self {
        Self::new(
            STALE_REVISION,
            format!("stale buffer revision: expected {expected}, current {actual}"),
        )
    }

    pub fn stale_buffer(expected: u64, actual: u64) -> Self {
        Self::new(
            STALE_REVISION,
            format!("stale buffer: expected {expected}, current {actual}"),
        )
    }

    pub fn invalid_position(message: impl Into<String>) -> Self {
        Self::new(INVALID_POSITION, message)
    }

    pub fn capability_denied(capability: &str, scope: Option<Value>) -> Self {
        let mut error = Self::new(
            CAPABILITY_DENIED,
            format!("capability denied: {capability}"),
        );
        let mut data = serde_json::Map::new();
        data.insert(
            "capability".to_string(),
            Value::String(capability.to_string()),
        );
        if let Some(scope) = scope {
            data.insert("scope".to_string(), scope);
        }
        error.data = Some(Value::Object(data));
        error
    }

    pub fn plugin(message: impl Into<String>) -> Self {
        Self::new(PLUGIN_ERROR, message)
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(PLUGIN_UNAVAILABLE, message)
    }

    pub fn ui_busy(message: impl Into<String>) -> Self {
        Self::new(UI_BUSY, message)
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    pub fn to_value(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("code".to_string(), Value::from(self.code));
        object.insert("message".to_string(), Value::from(self.message.clone()));
        if let Some(data) = &self.data {
            object.insert("data".to_string(), data.clone());
        }
        Value::Object(object)
    }
}

impl fmt::Display for RpcError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for RpcError {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn prompt_outcomes_serialize_with_kinds() {
        assert_eq!(
            serde_json::to_value(UiPromptOutcome::Text("Ada".to_string())).unwrap(),
            serde_json::json!({"kind": "text", "value": "Ada"})
        );
        assert_eq!(
            serde_json::to_value(UiPromptOutcome::Canceled).unwrap(),
            serde_json::json!({"kind": "canceled"})
        );
    }

    #[test]
    fn get_state_params_default_to_no_text() {
        let params: GetStateParams = serde_json::from_value(json!({})).unwrap();
        assert!(!params.include_text);
    }

    #[test]
    fn positions_round_trip_as_line_and_byte_column() {
        let position = Position {
            line: 3,
            byte_column: 9,
        };
        let value = serde_json::to_value(&position).unwrap();
        assert_eq!(value, json!({"line": 3, "byte_column": 9}));
        assert_eq!(serde_json::from_value::<Position>(value).unwrap(), position);
    }

    #[test]
    fn config_values_serialize_with_null_for_missing_keys() {
        let result = ConfigGetResult { value: None };
        assert_eq!(
            serde_json::to_value(result).unwrap(),
            json!({"value": null})
        );
        let result = ConfigListResult {
            settings: vec![ConfigSetting {
                key: "size".to_string(),
                value: json!(4),
            }],
        };
        assert_eq!(
            serde_json::to_value(result).unwrap(),
            json!({"settings": [{"key": "size", "value": 4}]})
        );
    }

    #[test]
    fn error_values_include_optional_data() {
        let error = RpcError::stale_revision(1, 2);
        assert_eq!(error.code, STALE_REVISION);
        assert!(error.message.contains("expected 1"));
        let value = error.to_value();
        assert_eq!(value["code"], STALE_REVISION);
    }
}
