use serde_json::Value;

use crate::error::PluginError;
use crate::protocol::{
    BufferChangedEvent, BufferClosedEvent, BufferOpenedEvent, DiagnosticSummary,
    DiagnosticsChangedEvent, FileOpenedEvent, FileSavedEvent, GetStateResult, ModeChangedEvent,
    Position, ProtocolTextEdit, RpcError, INTERNAL_ERROR, INVALID_PARAMS,
};

use super::bindings::kawi::plugin::types::{
    Argument, BufferChanged, BufferClosed, BufferOpened, Diagnostic, DiagnosticsChanged, Event,
    FileOpened, FileSaved, ModeChanged, Position as WitPosition, ProcessOutput, Severity, State,
};

pub fn argument_from_json(value: Value) -> Result<Argument, RpcError> {
    match value {
        Value::Null => Ok(Argument::Null),
        Value::Bool(value) => Ok(Argument::Boolean(value)),
        Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                return Ok(Argument::Integer(value));
            }
            let value = value
                .as_f64()
                .ok_or_else(|| RpcError::invalid_params("wasm arguments must be finite numbers"))?;
            if !value.is_finite() {
                return Err(RpcError::invalid_params(
                    "wasm arguments must be finite numbers",
                ));
            }
            Ok(Argument::Float(value))
        }
        Value::String(value) => Ok(Argument::Text(value)),
        _ => Err(RpcError::invalid_params(
            "wasm arguments must be null, boolean, integer, float, or text",
        )),
    }
}

pub fn argument_to_json(value: Argument) -> Result<Value, PluginError> {
    match value {
        Argument::Null => Ok(Value::Null),
        Argument::Boolean(value) => Ok(Value::Bool(value)),
        Argument::Integer(value) => Ok(Value::from(value)),
        Argument::Float(value) if value.is_finite() => Ok(Value::from(value)),
        Argument::Float(_) => Err(PluginError::Protocol(
            "wasm plugin returned a non-finite number".to_string(),
        )),
        Argument::Text(value) => Ok(Value::String(value)),
    }
}

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

pub fn event_from_json(event: &str, payload: Value) -> Result<Event, RpcError> {
    match event {
        "buffer_changed" => {
            let event: BufferChangedEvent = parse(payload)?;
            Ok(Event::BufferChanged(BufferChanged {
                buffer: event.buffer,
                revision: event.revision,
                previous_revision: event.previous_revision,
                origin: event.origin,
                full_resync: event.full_resync,
                changes: event
                    .changes
                    .into_iter()
                    .map(edit_from_protocol)
                    .collect::<Result<_, _>>()?,
            }))
        }
        "buffer_opened" => {
            let event: BufferOpenedEvent = parse(payload)?;
            Ok(Event::BufferOpened(BufferOpened {
                buffer: event.buffer,
                path: event.path,
                language: event.language,
                revision: event.revision,
            }))
        }
        "file_opened" => {
            let event: FileOpenedEvent = parse(payload)?;
            Ok(Event::FileOpened(FileOpened {
                buffer: event.buffer,
                path: event.path,
                language: event.language,
                revision: event.revision,
            }))
        }
        "buffer_closed" => {
            let event: BufferClosedEvent = parse(payload)?;
            Ok(Event::BufferClosed(BufferClosed {
                buffer: event.buffer,
                path: event.path,
            }))
        }
        "file_saved" => {
            let event: FileSavedEvent = parse(payload)?;
            Ok(Event::FileSaved(FileSaved {
                buffer: event.buffer,
                path: event.path,
                revision: event.revision,
            }))
        }
        "mode_changed" => {
            let event: ModeChangedEvent = parse(payload)?;
            Ok(Event::ModeChanged(ModeChanged {
                buffer: event.buffer,
                previous: event.from,
                next: event.to,
            }))
        }
        "diagnostics_changed" => {
            let event: DiagnosticsChangedEvent = parse(payload)?;
            Ok(Event::DiagnosticsChanged(DiagnosticsChanged {
                buffer: event.buffer,
                path: event.path,
                revision: event.revision,
                diagnostics: event
                    .diagnostics
                    .into_iter()
                    .map(diagnostic_from_protocol)
                    .collect(),
            }))
        }
        _ => Err(RpcError::invalid_params(format!(
            "unknown plugin event: {event}"
        ))),
    }
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

fn parse<T: serde::de::DeserializeOwned>(payload: Value) -> Result<T, RpcError> {
    serde_json::from_value(payload).map_err(|error| RpcError::invalid_params(error.to_string()))
}

fn edit_from_protocol(
    edit: ProtocolTextEdit,
) -> Result<super::bindings::kawi::plugin::types::Edit, RpcError> {
    Ok(super::bindings::kawi::plugin::types::Edit {
        start: position_from_protocol(edit.start)?,
        end: position_from_protocol(edit.end)?,
        text: edit.text,
    })
}

fn diagnostic_from_protocol(diagnostic: DiagnosticSummary) -> Diagnostic {
    Diagnostic {
        start: position_from_protocol_lossy(diagnostic.start),
        end: position_from_protocol_lossy(diagnostic.end),
        severity: severity_from_name(&diagnostic.severity),
        message: diagnostic.message,
        source: diagnostic.source,
        code: diagnostic.code,
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

fn position_to_protocol(position: WitPosition) -> Result<Position, RpcError> {
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
