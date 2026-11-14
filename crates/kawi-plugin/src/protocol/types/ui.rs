use serde::{Deserialize, Serialize};

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
