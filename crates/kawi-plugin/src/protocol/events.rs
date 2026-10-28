use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Position, ProtocolTextEdit};

pub const EVENT_NAMES: [&str; 7] = [
    "buffer_changed",
    "buffer_opened",
    "file_opened",
    "buffer_closed",
    "file_saved",
    "mode_changed",
    "diagnostics_changed",
];

pub const ORIGIN_USER: &str = "user";
pub const ORIGIN_UNDO: &str = "undo";
pub const ORIGIN_EXTERNAL_RELOAD: &str = "external_reload";
pub const ORIGIN_MIXED: &str = "mixed";

pub fn plugin_origin(plugin: &str) -> String {
    format!("plugin:{plugin}")
}

pub fn event_value(event: &str, payload: Value) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "event",
        "params": {
            "event": event,
            "payload": payload,
        },
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BufferChangedEvent {
    pub buffer: u64,
    pub revision: u64,
    pub previous_revision: u64,
    pub origin: String,
    pub full_resync: bool,
    pub changes: Vec<ProtocolTextEdit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BufferOpenedEvent {
    pub buffer: u64,
    pub path: Option<String>,
    pub language: Option<String>,
    pub revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileOpenedEvent {
    pub buffer: u64,
    pub path: String,
    pub language: Option<String>,
    pub revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BufferClosedEvent {
    pub buffer: u64,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileSavedEvent {
    pub buffer: u64,
    pub path: Option<String>,
    pub revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModeChangedEvent {
    pub buffer: u64,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticSummary {
    pub start: Position,
    pub end: Position,
    pub severity: String,
    pub message: String,
    pub source: Option<String>,
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticsChangedEvent {
    pub buffer: Option<u64>,
    pub path: String,
    pub revision: u64,
    pub diagnostics: Vec<DiagnosticSummary>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_values_have_a_name_and_payload() {
        let value = event_value("file_saved", serde_json::json!({"buffer": 1}));
        assert_eq!(value["method"], "event");
        assert_eq!(value["params"]["event"], "file_saved");
        assert_eq!(value["params"]["payload"]["buffer"], 1);
    }
}
