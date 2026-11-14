use serde::{Deserialize, Serialize};

use super::{Position, Selection};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BufferInfo {
    pub buffer: u64,
    pub revision: u64,
    pub path: Option<String>,
    pub language: Option<String>,
    pub modified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BufferState {
    pub buffer: u64,
    pub revision: u64,
    pub path: Option<String>,
    pub language: Option<String>,
    pub modified: bool,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BufferGetParams {
    pub buffer: u64,
    #[serde(default)]
    pub include_text: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BufferOpenParams {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowInfo {
    pub window: u64,
    pub buffer: u64,
    pub active: bool,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowParams {
    pub window: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowOpenParams {
    pub buffer: u64,
    pub direction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowCloseParams {
    pub window: u64,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetCursorParams {
    pub buffer: u64,
    pub cursor: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetSelectionParams {
    pub buffer: u64,
    pub selection: Option<Selection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetModeParams {
    pub buffer: u64,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct OptionsListParams {
    #[serde(default)]
    pub prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OptionsGetParams {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OptionInfo {
    pub name: String,
    pub aliases: Vec<String>,
    pub description: String,
    pub kind: String,
    pub scope: String,
    pub section: String,
    pub default: String,
    pub value: String,
    pub values: Vec<String>,
}
