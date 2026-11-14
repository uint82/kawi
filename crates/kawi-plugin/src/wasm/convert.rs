use serde_json::Value;

use crate::error::PluginError;
use crate::protocol::{
    DiagnosticSummary, GetStateResult, Position, ProtocolTextEdit, RpcError, UiPromptOutcome,
    INTERNAL_ERROR, INVALID_PARAMS,
};

use super::bindings::kawi::plugin::types::{
    Diagnostic, Position as WitPosition, ProcessOutput, PromptOutcome, Severity, State,
};

mod arguments;
mod plugin_api;
mod plugin_control;
mod visual;

pub use arguments::{argument_from_json, argument_to_json};
pub use plugin_api::{command_infos_from_json, setting_value_from_json, settings_from_json};
mod events;

pub use events::event_from_json;
pub use plugin_control::{
    buffer_infos_from_json, buffer_state_from_json, option_info_from_json, option_infos_from_json,
    selection_to_protocol, split_direction_name, window_info_from_json, window_infos_from_json,
};
pub(crate) use visual::decoration_style_to_protocol;
pub use visual::{decorations_to_protocol, signs_to_protocol, statusline_to_protocol};

pub fn state_from_json(value: Value) -> Result<State, RpcError> {
    let state: GetStateResult = serde_json::from_value(value)
        .map_err(|error| RpcError::invalid_params(error.to_string()))?;
    Ok(State {
        buffer: state.buffer,
        revision: state.revision,
        path: state.path,
        language: state.language,
        mode: state.mode,
        cursor: position_from_protocol(state.cursor)?,
        selection: state
            .selection
            .map(|selection| {
                Ok(super::bindings::kawi::plugin::types::Selection {
                    start: position_from_protocol(selection.start)?,
                    end: position_from_protocol(selection.end)?,
                })
            })
            .transpose()?,
        text: state.text,
    })
}

pub fn edits_to_protocol(
    edits: Vec<super::bindings::kawi::plugin::types::Edit>,
) -> Result<Vec<ProtocolTextEdit>, RpcError> {
    edits
        .into_iter()
        .map(|edit| {
            Ok(ProtocolTextEdit {
                start: position_to_protocol(edit.start)?,
                end: position_to_protocol(edit.end)?,
                text: edit.text,
            })
        })
        .collect()
}

pub fn diagnostics_to_protocol(diagnostics: Vec<Diagnostic>) -> Vec<DiagnosticSummary> {
    diagnostics
        .into_iter()
        .map(|diagnostic| DiagnosticSummary {
            start: position_to_protocol_lossy(diagnostic.start),
            end: position_to_protocol_lossy(diagnostic.end),
            severity: severity_name(diagnostic.severity).to_string(),
            message: diagnostic.message,
            source: diagnostic.source,
            code: diagnostic.code,
        })
        .collect()
}

pub fn rpc_from_wit_error(error: super::bindings::kawi::plugin::types::Error) -> PluginError {
    let mut rpc = RpcError::new(i64::from(error.code), error.message);
    if let Some(capability) = error.capability {
        rpc = rpc.with_data(serde_json::json!({ "capability": capability }));
    }
    PluginError::Rpc(rpc)
}

pub fn error_from_rpc(error: RpcError) -> super::bindings::kawi::plugin::types::Error {
    let capability = error
        .data
        .as_ref()
        .and_then(|data| data.get("capability"))
        .and_then(Value::as_str)
        .map(str::to_string);
    super::bindings::kawi::plugin::types::Error {
        code: i32::try_from(error.code).unwrap_or(INTERNAL_ERROR as i32),
        message: error.message,
        capability,
    }
}

pub fn process_output(stdout: String, stderr: String, code: Option<i32>) -> ProcessOutput {
    ProcessOutput {
        success: code.is_some_and(|code| code == 0),
        code,
        stdout,
        stderr,
    }
}

fn position_from_protocol(position: Position) -> Result<WitPosition, RpcError> {
    Ok(WitPosition {
        line: u32::try_from(position.line)
            .map_err(|_| RpcError::new(INVALID_PARAMS, "line exceeds u32"))?,
        byte_column: u32::try_from(position.byte_column)
            .map_err(|_| RpcError::new(INVALID_PARAMS, "column exceeds u32"))?,
    })
}

pub(super) fn position_to_protocol(position: WitPosition) -> Result<Position, RpcError> {
    Ok(Position {
        line: usize::try_from(position.line)
            .map_err(|_| RpcError::new(INVALID_PARAMS, "line does not fit usize"))?,
        byte_column: usize::try_from(position.byte_column)
            .map_err(|_| RpcError::new(INVALID_PARAMS, "column does not fit usize"))?,
    })
}

fn position_from_protocol_lossy(position: Position) -> WitPosition {
    WitPosition {
        line: u32::try_from(position.line).unwrap_or(u32::MAX),
        byte_column: u32::try_from(position.byte_column).unwrap_or(u32::MAX),
    }
}

fn position_to_protocol_lossy(position: WitPosition) -> Position {
    Position {
        line: position.line as usize,
        byte_column: position.byte_column as usize,
    }
}

pub fn prompt_outcome_to_wit(outcome: UiPromptOutcome) -> PromptOutcome {
    match outcome {
        UiPromptOutcome::Text(value) => PromptOutcome::Text(value),
        UiPromptOutcome::Selected(value) => PromptOutcome::Selected(value),
        UiPromptOutcome::Confirmed(value) => PromptOutcome::Confirmed(value),
        UiPromptOutcome::Canceled => PromptOutcome::Canceled,
    }
}

fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Information => "information",
        Severity::Hint => "hint",
    }
}

fn severity_from_name(name: &str) -> Severity {
    match name {
        "error" => Severity::Error,
        "warning" => Severity::Warning,
        "information" => Severity::Information,
        _ => Severity::Hint,
    }
}
