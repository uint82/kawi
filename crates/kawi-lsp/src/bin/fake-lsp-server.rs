use anyhow::{Context, Result};
use kawi_lsp::framing;
use kawi_lsp::protocol::{Message, RequestId, ResponseError};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::{self, BufReader};
use std::path::PathBuf;
use std::time::Duration;

const REQUEST_PROGRESS: i64 = 1001;
const REQUEST_CONFIG: i64 = 1002;
const REQUEST_REGISTER: i64 = 1003;
const REQUEST_FOLDERS: i64 = 1004;

fn main() -> Result<()> {
    let mut reader = BufReader::new(io::stdin());
    let mut writer = io::stdout();
    let mode = std::env::var("FAKE_LSP_SYNC").unwrap_or_else(|_| "full".to_string());
    let delay = std::env::var("FAKE_LSP_INIT_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    if let Ok(path) = std::env::var("FAKE_LSP_PID_FILE") {
        std::fs::write(PathBuf::from(path), std::process::id().to_string())
            .context("write fake server pid")?;
    }
    let mut pending = HashSet::new();
    let mut responses_valid = true;
    while let Some(value) = framing::read_message(&mut reader)? {
        let message = Message::parse(value)?;
        match message {
            Message::Request { id, method, params } => match method.as_str() {
                "initialize" => {
                    if delay > 0 {
                        std::thread::sleep(Duration::from_millis(delay));
                    }
                    let change = if mode == "incremental" { 2 } else { 1 };
                    let result = json!({
                        "capabilities": {
                            "positionEncoding": "utf-16",
                            "textDocumentSync": {
                                "openClose": true,
                                "change": change,
                                "save": {"includeText": true}
                            }
                        },
                        "serverInfo": {"name": "fake-lsp", "version": "1"}
                    });
                    send(&mut writer, Message::response(id, result))?;
                }
                "shutdown" => {
                    send(&mut writer, Message::response(id, Value::Null))?;
                }
                _ => {
                    send(
                        &mut writer,
                        Message::error(id, ResponseError::method_not_found(&method)),
                    )?;
                    let _ = params;
                }
            },
            Message::Notification { method, params } => match method.as_str() {
                "initialized" => {
                    send_requests(&mut writer, &mut pending)?;
                    send_progress(&mut writer)?;
                    send_log(&mut writer, "ready")?;
                }
                "exit" => return Ok(()),
                "textDocument/didOpen" => handle_open(&mut writer, params)?,
                "textDocument/didChange" => handle_change(&mut writer, params)?,
                "textDocument/didSave" => handle_save(&mut writer, params)?,
                "textDocument/didClose" => send_log(&mut writer, "close")?,
                _ => {}
            },
            Message::Response { id, result } => {
                let RequestId::Number(number) = id else {
                    continue;
                };
                if pending.remove(&number) {
                    let text = match result {
                        Ok(value) => format!("response:{number}:{value}"),
                        Err(error) => format!("response:{number}:error:{}", error.message),
                    };
                    if !valid_response(number, &text) {
                        responses_valid = false;
                    }
                    send_log(&mut writer, &text)?;
                    if pending.is_empty() {
                        let message = if responses_valid {
                            "requests:ok"
                        } else {
                            "requests:bad"
                        };
                        send_log(&mut writer, message)?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn valid_response(number: i64, text: &str) -> bool {
    match number {
        REQUEST_PROGRESS => text.ends_with(":null"),
        REQUEST_CONFIG => text.ends_with(":[\"yes\",null]"),
        REQUEST_REGISTER => text.ends_with(":null"),
        REQUEST_FOLDERS => text.contains("file://") && text.ends_with(']'),
        _ => true,
    }
}

fn send_requests(writer: &mut impl io::Write, pending: &mut HashSet<i64>) -> Result<()> {
    for id in [
        REQUEST_PROGRESS,
        REQUEST_CONFIG,
        REQUEST_REGISTER,
        REQUEST_FOLDERS,
    ] {
        pending.insert(id);
    }
    send(
        writer,
        Message::request(
            RequestId::Number(REQUEST_PROGRESS),
            "window/workDoneProgress/create",
            Some(json!({"token": "fake-progress"})),
        ),
    )?;
    send(
        writer,
        Message::request(
            RequestId::Number(REQUEST_CONFIG),
            "workspace/configuration",
            Some(json!({"items": [{"section": "fake"}, {"section": "missing"}]})),
        ),
    )?;
    send(
        writer,
        Message::request(
            RequestId::Number(REQUEST_REGISTER),
            "client/registerCapability",
            Some(json!({"registrations": []})),
        ),
    )?;
    send(
        writer,
        Message::request(
            RequestId::Number(REQUEST_FOLDERS),
            "workspace/workspaceFolders",
            None,
        ),
    )?;
    Ok(())
}

fn send_progress(writer: &mut impl io::Write) -> Result<()> {
    for value in [
        json!({"token": "fake-progress", "value": {"kind": "begin", "title": "Indexing", "percentage": 0}}),
        json!({"token": "fake-progress", "value": {"kind": "report", "message": "half", "percentage": 50}}),
        json!({"token": "fake-progress", "value": {"kind": "end", "message": "done"}}),
    ] {
        send(writer, Message::notification("$/progress", Some(value)))?;
    }
    Ok(())
}

fn handle_open(writer: &mut impl io::Write, params: Option<Value>) -> Result<()> {
    let params = params.context("didOpen params")?;
    let document = &params["textDocument"];
    let version = document["version"].as_i64().unwrap_or(0);
    let text = document["text"].as_str().unwrap_or("");
    send_log(writer, &format!("open:{version}:{text}"))
}

fn handle_change(writer: &mut impl io::Write, params: Option<Value>) -> Result<()> {
    let params = params.context("didChange params")?;
    let version = params["textDocument"]["version"].as_i64().unwrap_or(0);
    let change = &params["contentChanges"][0];
    let text = change["text"].as_str().unwrap_or("");
    let kind = if change.get("range").is_some() {
        "incremental"
    } else {
        "full"
    };
    send_log(writer, &format!("change:{version}:{kind}:{text}"))
}

fn handle_save(writer: &mut impl io::Write, params: Option<Value>) -> Result<()> {
    let params = params.context("didSave params")?;
    let text = params.get("text").and_then(Value::as_str).unwrap_or("");
    send_log(writer, &format!("save:{text}"))
}

fn send_log(writer: &mut impl io::Write, message: &str) -> Result<()> {
    send(
        writer,
        Message::notification(
            "window/logMessage",
            Some(json!({"type": 4, "message": message})),
        ),
    )
}

fn send(writer: &mut impl io::Write, message: Message) -> Result<()> {
    framing::write_message(writer, &message.to_value())
}
