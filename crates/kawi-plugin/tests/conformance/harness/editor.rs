use serde_json::{json, Value};

use kawi_plugin::{
    RpcError, MAX_DIAGNOSTIC_CODE_BYTES, MAX_DIAGNOSTIC_MESSAGE_BYTES, MAX_PLUGIN_DIAGNOSTICS,
};

const EDITOR_BUFFER: &str = "é\n";

pub(super) fn apply_edit(params: &Value) -> Result<Value, RpcError> {
    let edits = params
        .get("edits")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for edit in edits {
        validate_position(edit.get("start"))?;
        validate_position(edit.get("end"))?;
    }
    Ok(json!({ "revision": next_revision(params) }))
}

fn validate_position(position: Option<&Value>) -> Result<(), RpcError> {
    let line = position
        .and_then(|position| position.get("line"))
        .and_then(Value::as_u64)
        .unwrap_or_default() as usize;
    let column = position
        .and_then(|position| position.get("byte_column"))
        .and_then(Value::as_u64)
        .unwrap_or_default() as usize;
    let Some(text) = EDITOR_BUFFER.lines().nth(line) else {
        return Err(RpcError::invalid_position(
            "edit position is out of range".to_string(),
        ));
    };
    if column > text.len() {
        return Err(RpcError::invalid_position(
            "edit position is out of range".to_string(),
        ));
    }
    if !text.is_char_boundary(column) {
        return Err(RpcError::invalid_position(
            "edit position splits a UTF-8 codepoint".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn diagnostics_invalid(params: &Value) -> Option<RpcError> {
    let diagnostics = params.get("diagnostics").and_then(Value::as_array)?;
    if diagnostics.len() > MAX_PLUGIN_DIAGNOSTICS {
        return Some(RpcError::invalid_params(format!(
            "at most {MAX_PLUGIN_DIAGNOSTICS} diagnostics are accepted"
        )));
    }
    for diagnostic in diagnostics {
        if diagnostic
            .get("message")
            .and_then(Value::as_str)
            .is_some_and(|message| message.len() > MAX_DIAGNOSTIC_MESSAGE_BYTES)
        {
            return Some(RpcError::invalid_params(format!(
                "diagnostic message exceeds {MAX_DIAGNOSTIC_MESSAGE_BYTES} bytes"
            )));
        }
        if diagnostic
            .get("code")
            .and_then(Value::as_str)
            .is_some_and(|code| code.len() > MAX_DIAGNOSTIC_CODE_BYTES)
        {
            return Some(RpcError::invalid_params(format!(
                "diagnostic code exceeds {MAX_DIAGNOSTIC_CODE_BYTES} bytes"
            )));
        }
    }
    None
}

fn next_revision(params: &Value) -> u64 {
    params
        .get("revision")
        .and_then(Value::as_u64)
        .unwrap_or_default()
        .saturating_add(1)
}

pub(super) fn oversized_state_value() -> Value {
    let mut value = state_value();
    value["text"] = json!("x".repeat(kawi_jsonrpc::framing::MAX_MESSAGE_BYTES + 1));
    value
}

pub(super) fn state_value() -> Value {
    json!({
        "buffer": 1,
        "revision": 7,
        "path": "/tmp/conformance.rs",
        "language": "rust",
        "mode": "normal",
        "cursor": { "line": 0, "byte_column": 0 },
        "selection": null,
        "text": null,
    })
}
