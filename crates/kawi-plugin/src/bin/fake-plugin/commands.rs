use std::io::Write;

use anyhow::Result;
use serde_json::{json, Value};

use super::{host_request, respond, respond_error};

pub(super) fn host_call(
    message: &Value,
    reader: &mut impl std::io::BufRead,
    stdout: &mut impl Write,
) -> Result<()> {
    let value = host_request(
        "editor.get_state",
        json!({"include_text": true}),
        reader,
        stdout,
    )?;
    if let Some(error) = value.get("error") {
        return respond_error(stdout, message, error.clone());
    }
    respond(
        stdout,
        message,
        json!({
            "host_result": value.get("result").cloned().unwrap_or(Value::Null),
        }),
    )
}

pub(super) fn multi_call(
    message: &Value,
    reader: &mut impl std::io::BufRead,
    stdout: &mut impl Write,
) -> Result<()> {
    for (method, params) in [
        ("editor.get_state", json!({"include_text": false})),
        ("ui.message", json!({"message": "conformance"})),
        (
            "editor.set_diagnostics",
            json!({"buffer": 1, "revision": 7, "diagnostics": []}),
        ),
    ] {
        let value = host_request(method, params, reader, stdout)?;
        if let Some(error) = value.get("error") {
            return respond_error(stdout, message, error.clone());
        }
    }
    respond(stdout, message, json!({"ok": true}))
}

pub(super) fn diagnostics_call(
    message: &Value,
    reader: &mut impl std::io::BufRead,
    stdout: &mut impl Write,
) -> Result<()> {
    let command = message
        .get("params")
        .and_then(|params| params.get("command"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    if command != "diagnostics" {
        return respond(stdout, message, json!({"echo": message.get("params")}));
    }
    let value = host_request(
        "editor.set_diagnostics",
        json!({
            "buffer": 1,
            "revision": 7,
            "source": "x".repeat(1024 * 1024),
            "diagnostics": [],
        }),
        reader,
        stdout,
    )?;
    if let Some(error) = value.get("error") {
        return respond_error(stdout, message, error.clone());
    }
    respond(stdout, message, json!({"ok": true}))
}

pub(super) fn apply_edit(
    message: &Value,
    reader: &mut impl std::io::BufRead,
    stdout: &mut impl Write,
) -> Result<()> {
    let value = host_request(
        "editor.apply_edit",
        json!({"buffer": 1, "revision": 7, "edits": []}),
        reader,
        stdout,
    )?;
    if let Some(error) = value.get("error") {
        return respond_error(stdout, message, error.clone());
    }
    respond(stdout, message, json!({"ok": true}))
}

pub(super) fn apply_edit_mid_codepoint(
    message: &Value,
    reader: &mut impl std::io::BufRead,
    stdout: &mut impl Write,
) -> Result<()> {
    let value = host_request(
        "editor.apply_edit",
        json!({
            "buffer": 1,
            "revision": 7,
            "edits": [{
                "start": { "line": 0, "byte_column": 1 },
                "end": { "line": 0, "byte_column": 1 },
                "text": "",
            }],
        }),
        reader,
        stdout,
    )?;
    if let Some(error) = value.get("error") {
        return respond_error(stdout, message, error.clone());
    }
    respond(stdout, message, json!({"ok": true}))
}

pub(super) fn read_file(message: &Value, stdout: &mut impl Write) -> Result<()> {
    let path = message
        .get("params")
        .and_then(|params| params.get("args"))
        .and_then(|args| args.get("path"))
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing path"))?;
    let result = match std::fs::read_to_string(path) {
        Ok(content) => json!({"content": content}),
        Err(error) => json!({"error": error.to_string()}),
    };
    respond(stdout, message, result)
}

pub(super) fn exec_command(message: &Value, stdout: &mut impl Write) -> Result<()> {
    let path = message
        .get("params")
        .and_then(|params| params.get("args"))
        .and_then(|args| args.get("path"))
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing path"))?;
    let result = match std::process::Command::new(path).status() {
        Ok(status) => json!({"success": status.success()}),
        Err(error) => json!({"error": error.to_string()}),
    };
    respond(stdout, message, result)
}
