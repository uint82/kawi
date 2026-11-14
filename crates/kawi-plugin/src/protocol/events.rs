use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Position, ProtocolTextEdit, Selection};

pub const EVENT_NAMES: [&str; 15] = [
    "buffer_changed",
    "buffer_opened",
    "file_opened",
    "buffer_closed",
    "file_saved",
    "mode_changed",
    "diagnostics_changed",
    "buffer_enter",
    "buffer_leave",
    "window_enter",
    "window_leave",
    "cursor_moved",
    "selection_changed",
    "focus_gained",
    "focus_lost",
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

    #[test]
    fn new_event_payloads_round_trip() {
        let position = Position {
            line: 2,
            byte_column: 4,
        };
        let selection = Selection {
            start: position.clone(),
            end: Position {
                line: 3,
                byte_column: 0,
            },
        };
        let payloads = [
            serde_json::to_value(BufferEnterEvent {
                buffer: 1,
                path: Some("src/main.rs".to_string()),
                language: Some("rust".to_string()),
                revision: 7,
            })
            .expect("buffer enter"),
            serde_json::to_value(BufferLeaveEvent {
                buffer: 1,
                path: None,
            })
            .expect("buffer leave"),
            serde_json::to_value(WindowEnterEvent {
                window: 2,
                buffer: 1,
            })
            .expect("window enter"),
            serde_json::to_value(WindowLeaveEvent {
                window: 2,
                buffer: 1,
            })
            .expect("window leave"),
            serde_json::to_value(CursorMovedEvent {
                buffer: 1,
                cursor: position,
                selection: Some(selection.clone()),
            })
            .expect("cursor moved"),
            serde_json::to_value(SelectionChangedEvent {
                buffer: 1,
                selection: Some(selection),
            })
            .expect("selection changed"),
            serde_json::to_value(FocusChangedEvent {
                window: 2,
                buffer: 1,
            })
            .expect("focus changed"),
        ];
        for payload in payloads {
            let wrapped = event_value("new_event", payload.clone());
            assert_eq!(wrapped["params"]["payload"], payload);
            assert_eq!(
                serde_json::from_value::<serde_json::Value>(payload).expect("payload"),
                serde_json::from_value::<serde_json::Value>(wrapped["params"]["payload"].clone())
                    .expect("wrapped payload")
            );
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BufferEnterEvent {
    pub buffer: u64,
    pub path: Option<String>,
    pub language: Option<String>,
    pub revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BufferLeaveEvent {
    pub buffer: u64,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowEnterEvent {
    pub window: u64,
    pub buffer: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowLeaveEvent {
    pub window: u64,
    pub buffer: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CursorMovedEvent {
    pub buffer: u64,
    pub cursor: Position,
    pub selection: Option<Selection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectionChangedEvent {
    pub buffer: u64,
    pub selection: Option<Selection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FocusChangedEvent {
    pub window: u64,
    pub buffer: u64,
}
