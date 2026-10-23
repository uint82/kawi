use anyhow::{Context, Result};
use kawi_lsp::framing;
use kawi_lsp::protocol::{Message, RequestId, ResponseError};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
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
    let navigation_delay = std::env::var("FAKE_LSP_NAV_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    let navigation_line_offset = std::env::var("FAKE_LSP_NAV_LINE_OFFSET")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0);
    let rename_mode =
        std::env::var("FAKE_LSP_RENAME_MODE").unwrap_or_else(|_| "prepare".to_string());
    let rename_delay = std::env::var("FAKE_LSP_RENAME_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    if let Ok(path) = std::env::var("FAKE_LSP_PID_FILE") {
        std::fs::write(PathBuf::from(path), std::process::id().to_string())
            .context("write fake server pid")?;
    }
    let mut pending = HashSet::new();
    let mut documents: HashMap<String, i64> = HashMap::new();
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
                    let rename = if rename_mode == "simple" {
                        json!(true)
                    } else {
                        json!({"prepareProvider": true})
                    };
                    let result = json!({
                        "capabilities": {
                            "positionEncoding": "utf-16",
                            "textDocumentSync": {
                                "openClose": true,
                                "change": change,
                                "save": {"includeText": true}
                            },
                            "definitionProvider": true,
                            "referencesProvider": true,
                            "hoverProvider": true,
                            "renameProvider": rename
                        },
                        "serverInfo": {"name": "fake-lsp", "version": "1"}
                    });
                    send(&mut writer, Message::response(id, result))?;
                }
                "textDocument/definition" | "textDocument/references" | "textDocument/hover" => {
                    handle_navigation(
                        &mut writer,
                        id,
                        &method,
                        params,
                        navigation_delay,
                        navigation_line_offset,
                    )?;
                }
                "textDocument/prepareRename" => {
                    handle_prepare_rename(&mut writer, id, params, rename_delay, &rename_mode)?
                }
                "textDocument/rename" => handle_rename(
                    &mut writer,
                    id,
                    params,
                    rename_delay,
                    &rename_mode,
                    &documents,
                )?,
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
                "textDocument/didOpen" => handle_open(&mut writer, params, &mut documents)?,
                "textDocument/didChange" => handle_change(&mut writer, params)?,
                "textDocument/didSave" => handle_save(&mut writer, params)?,
                "textDocument/didClose" => send_log(&mut writer, "close")?,
                "$/cancelRequest" => {
                    let id = params
                        .as_ref()
                        .and_then(|params| params.get("id"))
                        .map_or_else(String::new, Value::to_string);
                    send_log(&mut writer, &format!("cancel:{id}"))?;
                }
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

fn handle_open(
    writer: &mut impl io::Write,
    params: Option<Value>,
    documents: &mut HashMap<String, i64>,
) -> Result<()> {
    let params = params.context("didOpen params")?;
    let document = &params["textDocument"];
    let version = document["version"].as_i64().unwrap_or(0);
    let text = document["text"].as_str().unwrap_or("");
    let uri = document["uri"].as_str().unwrap_or("");
    documents.insert(uri.to_string(), version);
    send_log(writer, &format!("open:{version}:{text}"))?;
    send_diagnostics(writer, uri, Some(version), "open-diagnostic")
}

fn handle_prepare_rename(
    writer: &mut impl io::Write,
    id: RequestId,
    params: Option<Value>,
    delay: u64,
    mode: &str,
) -> Result<()> {
    let params = params.context("prepare rename params")?;
    let position = &params["position"];
    let line = position["line"].as_i64().unwrap_or(0);
    let character = position["character"].as_i64().unwrap_or(0);
    send_log(writer, &format!("prepare-rename:{line}:{character}"))?;
    if delay > 0 {
        std::thread::sleep(Duration::from_millis(delay));
    }
    let result = if mode == "empty" {
        Value::Null
    } else {
        json!({
            "range": range(line, character, character.saturating_add(3)),
            "placeholder": "fake_symbol"
        })
    };
    send(writer, Message::response(id, result))
}

fn handle_rename(
    writer: &mut impl io::Write,
    id: RequestId,
    params: Option<Value>,
    delay: u64,
    mode: &str,
    documents: &HashMap<String, i64>,
) -> Result<()> {
    let params = params.context("rename params")?;
    let uri = params["textDocument"]["uri"]
        .as_str()
        .context("rename uri")?
        .to_string();
    let new_name = params["newName"].as_str().unwrap_or("renamed").to_string();
    let position = &params["position"];
    let line = position["line"].as_i64().unwrap_or(0);
    let character = position["character"].as_i64().unwrap_or(0);
    send_log(writer, &format!("rename:{new_name}:{line}:{character}"))?;
    if delay > 0 {
        std::thread::sleep(Duration::from_millis(delay));
    }
    let other = sibling_uri(&uri, "other.rs");
    let result = match mode {
        "resource" => json!({
            "documentChanges": [{
                "kind": "create",
                "uri": other
            }]
        }),
        "mismatch" => json!({
            "documentChanges": [text_document_edit(
                &uri,
                Some(999),
                line,
                character,
                character.saturating_add(4),
                &new_name
            )]
        }),
        _ => json!({
            "documentChanges": [
                text_document_edit(
                    &uri,
                    documents.get(&uri).copied(),
                    line,
                    character,
                    character.saturating_add(4),
                    &new_name
                ),
                text_document_edit(&other, None, 0, 3, 8, &new_name)
            ]
        }),
    };
    send(writer, Message::response(id, result))
}

fn text_document_edit(
    uri: &str,
    version: Option<i64>,
    line: i64,
    start: i64,
    end: i64,
    new_name: &str,
) -> Value {
    json!({
        "textDocument": {"uri": uri, "version": version},
        "edits": [{
            "range": range(line, start, end),
            "newText": new_name
        }]
    })
}

fn range(line: i64, start: i64, end: i64) -> Value {
    json!({
        "start": {"line": line, "character": start},
        "end": {"line": line, "character": end}
    })
}

fn sibling_uri(uri: &str, name: &str) -> String {
    match uri.rsplit_once('/') {
        Some((prefix, _)) => format!("{prefix}/{name}"),
        None => uri.to_string(),
    }
}

fn handle_change(writer: &mut impl io::Write, params: Option<Value>) -> Result<()> {
    let params = params.context("didChange params")?;
    let version = params["textDocument"]["version"].as_i64().unwrap_or(0);
    let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
    let change = &params["contentChanges"][0];
    let text = change["text"].as_str().unwrap_or("");
    if let Some(range) = change.get("range") {
        let start = &range["start"];
        let end = &range["end"];
        send_log(
            writer,
            &format!(
                "change:{version}:incremental:{}:{}:{}:{}:{text}",
                start["line"].as_i64().unwrap_or(0),
                start["character"].as_i64().unwrap_or(0),
                end["line"].as_i64().unwrap_or(0),
                end["character"].as_i64().unwrap_or(0)
            ),
        )?;
    } else {
        send_log(writer, &format!("change:{version}:full:{text}"))?;
    }
    if version > 1 {
        send_diagnostics(writer, uri, Some(version.saturating_sub(1)), "stale")?;
    }
    send_diagnostics(writer, uri, Some(version), "fresh")
}

fn send_diagnostics(
    writer: &mut impl io::Write,
    uri: &str,
    version: Option<i64>,
    message: &str,
) -> Result<()> {
    let mut params = json!({
        "uri": uri,
        "diagnostics": [{
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 0, "character": 1}
            },
            "severity": 1,
            "source": "fake",
            "message": message
        }]
    });
    if let Some(version) = version {
        params["version"] = json!(version);
    }
    send(
        writer,
        Message::notification("textDocument/publishDiagnostics", Some(params)),
    )
}

fn handle_navigation(
    writer: &mut impl io::Write,
    id: RequestId,
    method: &str,
    params: Option<Value>,
    delay: u64,
    line_offset: i64,
) -> Result<()> {
    let params = params.context("navigation params")?;
    let uri = params["textDocument"]["uri"]
        .as_str()
        .context("navigation uri")?
        .to_string();
    let position = &params["position"];
    let line = position["line"].as_i64().unwrap_or(0) + line_offset;
    let character = position["character"].as_i64().unwrap_or(0);
    send_log(writer, &format!("nav:{method}:{line}:{character}"))?;
    if delay > 0 {
        std::thread::sleep(Duration::from_millis(delay));
    }
    let result = match method {
        "textDocument/definition" => json!([
            location(&uri, line, character),
            location(&uri, line.saturating_add(1), 0)
        ]),
        "textDocument/references" => json!([
            location(&uri, line, character),
            location(&uri, line.saturating_add(1), 0)
        ]),
        _ => json!({
            "contents": {
                "kind": "markdown",
                "value": "```rust\nfn fake()\n```\n---\nFake hover"
            }
        }),
    };
    send(writer, Message::response(id, result))
}

fn location(uri: &str, line: i64, character: i64) -> Value {
    json!({
        "uri": uri,
        "range": {
            "start": {"line": line, "character": character},
            "end": {"line": line, "character": character.saturating_add(1)}
        }
    })
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
