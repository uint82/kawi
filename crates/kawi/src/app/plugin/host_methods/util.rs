use kawi_editor::Mode;

use kawi_plugin::{RpcError, INTERNAL_ERROR};
use serde_json::Value;

pub(crate) fn json_result<T: serde::Serialize>(value: T) -> Result<Value, RpcError> {
    serde_json::to_value(value).map_err(|error| RpcError::new(INTERNAL_ERROR, error.to_string()))
}

pub(crate) fn invalid_params(error: serde_json::Error) -> RpcError {
    RpcError::invalid_params(error.to_string())
}

pub(crate) fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Normal => "normal",
        Mode::Insert => "insert",
        Mode::Replace => "replace",
        Mode::Command => "command",
        Mode::Visual | Mode::VisualLine | Mode::VisualBlock => "visual",
        Mode::Search => "search",
    }
}
