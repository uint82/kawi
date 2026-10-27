use kawi_editor::FoldMethod;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionFoldMethod {
    Manual,
    Indent,
}

impl From<FoldMethod> for SessionFoldMethod {
    fn from(method: FoldMethod) -> Self {
        match method {
            FoldMethod::Manual => Self::Manual,
            FoldMethod::Indent => Self::Indent,
        }
    }
}

impl From<SessionFoldMethod> for FoldMethod {
    fn from(method: SessionFoldMethod) -> Self {
        match method {
            SessionFoldMethod::Manual => Self::Manual,
            SessionFoldMethod::Indent => Self::Indent,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionFold {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionFolds {
    pub method: SessionFoldMethod,
    pub content_hash: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub manual: Vec<SessionFold>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub closed: Vec<usize>,
}

impl SessionFolds {
    pub(super) fn validate(&self) -> Result<(), String> {
        if self.manual.iter().any(|fold| fold.start >= fold.end) {
            return Err("fold range is empty or reversed".to_string());
        }
        Ok(())
    }
}
