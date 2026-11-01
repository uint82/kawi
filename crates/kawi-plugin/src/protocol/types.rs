use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::events::DiagnosticSummary;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub byte_column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Selection {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InvocationContext {
    pub buffer: u64,
    pub revision: u64,
    pub path: Option<String>,
    pub language: Option<String>,
    pub mode: String,
    pub cursor: Position,
    pub selection: Option<Selection>,
}

impl InvocationContext {
    pub fn redacted() -> Self {
        Self {
            buffer: 0,
            revision: 0,
            path: None,
            language: None,
            mode: "normal".to_string(),
            cursor: Position {
                line: 0,
                byte_column: 0,
            },
            selection: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokeParams {
    pub command: String,
    pub args: Value,
    pub context: InvocationContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    pub protocol_version: u64,
    pub protocol_minor: u64,
    pub host: HostInfo,
    pub position_encoding: String,
    pub host_capabilities: Vec<String>,
    pub plugin_capabilities: Vec<String>,
    pub events: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    pub protocol_version: u64,
    #[serde(default)]
    pub protocol_minor: u64,
    pub name: String,
    pub version: String,
    pub plugin_capabilities: Vec<String>,
    pub host_capabilities: Vec<String>,
    #[serde(default)]
    pub events: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct GetStateParams {
    pub include_text: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetStateResult {
    pub buffer: u64,
    pub revision: u64,
    pub path: Option<String>,
    pub language: Option<String>,
    pub mode: String,
    pub cursor: Position,
    pub selection: Option<Selection>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProtocolTextEdit {
    pub start: Position,
    pub end: Position,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyEditParams {
    pub buffer: u64,
    pub revision: u64,
    pub edits: Vec<ProtocolTextEdit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyEditResult {
    pub revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorExecuteParams {
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorCommandInfo {
    pub id: String,
    pub description: String,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorListCommandsResult {
    pub commands: Vec<EditorCommandInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigGetParams {
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigGetResult {
    pub value: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigSetting {
    pub key: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigListResult {
    pub settings: Vec<ConfigSetting>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiMessageParams {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiInputParams {
    pub title: String,
    pub placeholder: Option<String>,
    pub value: Option<String>,
    pub max_length: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiSelectParams {
    pub title: String,
    pub options: Vec<String>,
    pub selected: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiConfirmParams {
    pub title: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum UiPromptOutcome {
    Text(String),
    Selected(String),
    Confirmed(bool),
    Canceled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiProgressUpdateResult {
    pub canceled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiProgressStartParams {
    pub title: String,
    pub message: Option<String>,
    pub fraction: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiProgressOpenResult {
    pub id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiProgressUpdateParams {
    pub id: u64,
    pub message: Option<String>,
    pub fraction: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiProgressFinishParams {
    pub id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetDiagnosticsParams {
    pub buffer: u64,
    pub revision: u64,
    pub diagnostics: Vec<DiagnosticSummary>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecorationStyle {
    Accent,
    Info,
    Success,
    Warning,
    Error,
    Muted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Decoration {
    Highlight {
        start: Position,
        end: Position,
        style: DecorationStyle,
    },
    VirtualText {
        line: usize,
        text: String,
        style: DecorationStyle,
    },
    Sign {
        line: usize,
        text: String,
        style: DecorationStyle,
    },
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecorationScope {
    #[default]
    All,
    Visual,
    Signs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetDecorationsParams {
    pub buffer: u64,
    pub revision: u64,
    #[serde(default)]
    pub scope: DecorationScope,
    pub decorations: Vec<Decoration>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatuslineSegment {
    pub text: String,
    pub style: DecorationStyle,
    pub priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetStatuslineParams {
    pub segments: Vec<StatuslineSegment>,
}
