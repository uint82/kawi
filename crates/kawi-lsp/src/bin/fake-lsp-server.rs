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
const REQUEST_REFRESH: i64 = 1005;

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
    let pull_diagnostics =
        std::env::var("FAKE_LSP_PULL_DIAGNOSTICS").is_ok_and(|value| value == "1");
    let refresh_diagnostics =
        std::env::var("FAKE_LSP_REFRESH_DIAGNOSTICS").is_ok_and(|value| value == "1");
    let workspace_diagnostics =
        std::env::var("FAKE_LSP_WORKSPACE_DIAGNOSTICS").is_ok_and(|value| value == "1");
    let file_operations = std::env::var("FAKE_LSP_FILE_OPERATIONS").is_ok_and(|value| value == "1");
    let symbol_providers = std::env::var("FAKE_LSP_SYMBOL_PROVIDERS")
        .is_ok_and(|value| value == "1")
        || std::env::var("FAKE_LSP_SYMBOL_PROVIDERS").is_err();
    let symbol_resolve = std::env::var("FAKE_LSP_SYMBOL_RESOLVE").is_ok_and(|value| value == "1")
        || std::env::var("FAKE_LSP_SYMBOL_RESOLVE").is_err();
    let symbol_mode =
        std::env::var("FAKE_LSP_SYMBOL_MODE").unwrap_or_else(|_| "nested".to_string());
    let symbol_delay = std::env::var("FAKE_LSP_SYMBOL_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    let symbol_line_offset = std::env::var("FAKE_LSP_SYMBOL_LINE_OFFSET")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0);
    let symbol_settings = SymbolSettings {
        mode: &symbol_mode,
        delay: symbol_delay,
        line_offset: symbol_line_offset,
        resolve: symbol_resolve,
    };
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
                    let mut capabilities = json!({
                        "positionEncoding": "utf-16",
                        "textDocumentSync": {
                            "openClose": true,
                            "change": change,
                            "save": {"includeText": true}
                        },
                        "definitionProvider": true,
                        "referencesProvider": true,
                        "hoverProvider": true,
                        "renameProvider": rename,
                        "completionProvider": {
                            "triggerCharacters": ["."],
                            "resolveProvider": true
                        },
                        "signatureHelpProvider": {"triggerCharacters": ["(", ","]},
                        "codeActionProvider": {"codeActionKinds": ["quickfix"]},
                        "documentFormattingProvider": true,
                        "documentRangeFormattingProvider": true
                    });
                    if pull_diagnostics || workspace_diagnostics {
                        capabilities["diagnosticProvider"] = json!({
                            "interFileDependencies": workspace_diagnostics,
                            "workspaceDiagnostics": workspace_diagnostics
                        });
                    }
                    if symbol_providers {
                        capabilities["documentSymbolProvider"] = json!(true);
                        capabilities["workspaceSymbolProvider"] = json!({
                            "resolveProvider": symbol_resolve
                        });
                    }
                    if file_operations {
                        capabilities["workspace"] = json!({
                            "fileOperations": {
                                "didRename": {"filters": [{"pattern": {"glob": "**/*"}}]},
                                "willRename": {"filters": [{"pattern": {"glob": "**/*"}}]},
                                "didDelete": {"filters": [{"pattern": {"glob": "**/*"}}]},
                                "willDelete": {"filters": [{"pattern": {"glob": "**/*"}}]}
                            }
                        });
                    }
                    let result = json!({
                        "capabilities": capabilities,
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
                "textDocument/completion" => handle_completion(&mut writer, id, params)?,
                "completionItem/resolve" => handle_completion_resolve(&mut writer, id, params)?,
                "textDocument/signatureHelp" => handle_signature_help(&mut writer, id, params)?,
                "textDocument/codeAction" => handle_code_actions(&mut writer, id, params)?,
                "textDocument/formatting" | "textDocument/rangeFormatting" => {
                    handle_formatting(&mut writer, id, &method, params)?
                }
                "textDocument/documentSymbol" => {
                    handle_document_symbols(&mut writer, id, params, &symbol_settings)?
                }
                "workspace/symbol" => {
                    handle_workspace_symbols(&mut writer, id, params, &documents, &symbol_settings)?
                }
                "workspaceSymbol/resolve" => {
                    handle_workspace_symbol_resolve(&mut writer, id, params, symbol_line_offset)?
                }
                "workspace/willRenameFiles" => handle_will_rename(&mut writer, id, params)?,
                "workspace/willDeleteFiles" => {
                    send(&mut writer, Message::response(id, Value::Null))?;
                }
                "workspace/executeCommand" => handle_execute_command(&mut writer, id, params)?,
                "textDocument/diagnostic" => {
                    handle_pull_diagnostic(&mut writer, id, params, &documents)?
                }
                "workspace/diagnostic" => {
                    handle_workspace_diagnostic(&mut writer, id, params, &documents)?
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
                    send_requests(&mut writer, &mut pending, refresh_diagnostics)?;
                    send_progress(&mut writer)?;
                    send_log(&mut writer, "ready")?;
                }
                "exit" => return Ok(()),
                "textDocument/didOpen" => handle_open(&mut writer, params, &mut documents)?,
                "textDocument/didChange" => handle_change(&mut writer, params, &mut documents)?,
                "textDocument/didSave" => handle_save(&mut writer, params)?,
                "textDocument/didClose" => send_log(&mut writer, "close")?,
                "workspace/didRenameFiles" => {
                    let old = file_uri_field(params.as_ref(), "oldUri");
                    let new = file_uri_field(params.as_ref(), "newUri");
                    send_log(&mut writer, &format!("didRename:{old}:{new}"))?;
                }
                "workspace/didDeleteFiles" => {
                    let uri = file_uri_field(params.as_ref(), "uri");
                    send_log(&mut writer, &format!("didDelete:{uri}"))?;
                }
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
        REQUEST_REFRESH => text.ends_with(":null"),
        _ => true,
    }
}

fn send_requests(
    writer: &mut impl io::Write,
    pending: &mut HashSet<i64>,
    refresh_diagnostics: bool,
) -> Result<()> {
    let mut requests = vec![
        REQUEST_PROGRESS,
        REQUEST_CONFIG,
        REQUEST_REGISTER,
        REQUEST_FOLDERS,
    ];
    if refresh_diagnostics {
        requests.push(REQUEST_REFRESH);
    }
    for id in requests {
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
    if refresh_diagnostics {
        send(
            writer,
            Message::request(
                RequestId::Number(REQUEST_REFRESH),
                "workspace/diagnostic/refresh",
                None,
            ),
        )?;
    }
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

fn handle_change(
    writer: &mut impl io::Write,
    params: Option<Value>,
    documents: &mut HashMap<String, i64>,
) -> Result<()> {
    let params = params.context("didChange params")?;
    let version = params["textDocument"]["version"].as_i64().unwrap_or(0);
    let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
    documents.insert(uri.to_string(), version);
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

struct SymbolSettings<'a> {
    mode: &'a str,
    delay: u64,
    line_offset: i64,
    resolve: bool,
}

fn handle_document_symbols(
    writer: &mut impl io::Write,
    id: RequestId,
    params: Option<Value>,
    settings: &SymbolSettings<'_>,
) -> Result<()> {
    let params = params.context("document symbol params")?;
    let uri = params["textDocument"]["uri"]
        .as_str()
        .context("document symbol uri")?;
    send_log(writer, &format!("document-symbols:{uri}"))?;
    if settings.delay > 0 {
        std::thread::sleep(Duration::from_millis(settings.delay));
    }
    let line_offset = settings.line_offset;
    let result = match settings.mode {
        "null" => Value::Null,
        "flat" => json!([{
            "name": "flat_symbol",
            "kind": 12,
            "location": location(uri, 1 + line_offset, 4),
            "containerName": "module"
        }]),
        _ => json!([
            {
                "name": "outer",
                "detail": "fn outer()",
                "kind": 12,
                "range": symbol_range(line_offset, 0, 2 + line_offset, 1),
                "selectionRange": symbol_range(line_offset, 3, line_offset, 8),
                "children": [{
                    "name": "inner",
                    "kind": 13,
                    "range": symbol_range(1 + line_offset, 0, 1 + line_offset, 10),
                    "selectionRange": symbol_range(1 + line_offset, 4, 1 + line_offset, 9)
                }]
            },
            {
                "name": "old_struct",
                "kind": 23,
                "tags": [1],
                "range": symbol_range(3 + line_offset, 0, 3 + line_offset, 10),
                "selectionRange": symbol_range(3 + line_offset, 5, 3 + line_offset, 15)
            },
            {
                "name": "future",
                "kind": 999,
                "range": symbol_range(4 + line_offset, 0, 4 + line_offset, 6),
                "selectionRange": symbol_range(4 + line_offset, 0, 4 + line_offset, 6)
            }
        ]),
    };
    send(writer, Message::response(id, result))
}

fn handle_workspace_symbols(
    writer: &mut impl io::Write,
    id: RequestId,
    params: Option<Value>,
    documents: &HashMap<String, i64>,
    settings: &SymbolSettings<'_>,
) -> Result<()> {
    let params = params.context("workspace symbol params")?;
    let query = params["query"].as_str().unwrap_or("");
    send_log(writer, &format!("workspace-symbols:{query}"))?;
    if settings.delay > 0 {
        std::thread::sleep(Duration::from_millis(settings.delay));
    }
    let Some(uri) = documents.keys().min() else {
        return send(writer, Message::response(id, Value::Null));
    };
    let line_offset = settings.line_offset;
    let result = match settings.mode {
        "null" => Value::Null,
        "flat" => json!([{
            "name": "workspace_flat",
            "kind": 12,
            "location": location(uri, 2 + line_offset, 1),
            "containerName": "workspace"
        }]),
        _ => {
            let mut symbols = vec![
                json!({
                    "name": "workspace_fn",
                    "kind": 12,
                    "containerName": "module",
                    "location": location(uri, 1 + line_offset, 0)
                }),
                json!({
                    "name": "remote_symbol",
                    "kind": 12,
                    "location": {"uri": "jar:file:///tmp/fake.jar!/Remote.class"}
                }),
            ];
            if settings.resolve {
                symbols.push(json!({
                    "name": "lazy_symbol",
                    "kind": 12,
                    "containerName": "module",
                    "location": {"uri": uri},
                    "data": {"symbol": "lazy"}
                }));
            }
            Value::Array(symbols)
        }
    };
    send(writer, Message::response(id, result))
}

fn handle_workspace_symbol_resolve(
    writer: &mut impl io::Write,
    id: RequestId,
    params: Option<Value>,
    line_offset: i64,
) -> Result<()> {
    let mut symbol = params.context("workspace resolve params")?;
    send_log(writer, "workspace-resolve")?;
    let uri = symbol["location"]["uri"]
        .as_str()
        .context("workspace resolve uri")?
        .to_string();
    symbol["location"] = json!({
        "uri": uri,
        "range": symbol_range(2 + line_offset, 2, 2 + line_offset, 8)
    });
    send(writer, Message::response(id, symbol))
}

fn symbol_range(start_line: i64, start_character: i64, end_line: i64, end_character: i64) -> Value {
    json!({
        "start": {"line": start_line, "character": start_character},
        "end": {"line": end_line, "character": end_character}
    })
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

fn handle_completion(
    writer: &mut impl io::Write,
    id: RequestId,
    params: Option<Value>,
) -> Result<()> {
    let params = params.context("completion params")?;
    let position = &params["position"];
    let line = position["line"].as_i64().unwrap_or(0);
    let character = position["character"].as_i64().unwrap_or(0);
    send_log(writer, &format!("completion:{line}:{character}"))?;
    let result = json!({
        "isIncomplete": false,
        "items": [
            {
                "label": "fake_edit",
                "detail": "edit",
                "sortText": "1",
                "textEdit": {
                    "range": range(line, character.saturating_sub(3), character),
                    "newText": "edited"
                },
                "additionalTextEdits": [{
                    "range": range(1, 0, 0),
                    "newText": "// extra\n"
                }]
            },
            {
                "label": "fake_snippet",
                "detail": "snippet",
                "sortText": "2",
                "insertText": "print(${1:value})",
                "insertTextFormat": 2
            },
            {
                "label": "fake_value",
                "detail": "value",
                "sortText": "3",
                "insertText": "fake_value"
            }
        ]
    });
    send(writer, Message::response(id, result))
}

fn handle_completion_resolve(
    writer: &mut impl io::Write,
    id: RequestId,
    params: Option<Value>,
) -> Result<()> {
    let mut item = params.context("completion resolve params")?;
    send_log(writer, "resolve")?;
    item["detail"] = json!("resolved detail");
    item["documentation"] = json!({
        "kind": "markdown",
        "value": "**resolved** documentation"
    });
    item["additionalTextEdits"] = json!([{
        "range": range(0, 0, 0),
        "newText": "// resolved\n"
    }]);
    send(writer, Message::response(id, item))
}

fn handle_signature_help(
    writer: &mut impl io::Write,
    id: RequestId,
    params: Option<Value>,
) -> Result<()> {
    let params = params.context("signature help params")?;
    let position = &params["position"];
    let line = position["line"].as_i64().unwrap_or(0);
    let character = position["character"].as_i64().unwrap_or(0);
    send_log(writer, &format!("signature:{line}:{character}"))?;
    send(
        writer,
        Message::response(
            id,
            json!({
                "signatures": [{
                    "label": "fake(value: i32, other: i32)",
                    "documentation": {
                        "kind": "markdown",
                        "value": "Fake signature"
                    },
                    "parameters": [
                        {"label": [5, 10]},
                        {"label": [17, 22]}
                    ]
                }],
                "activeSignature": 0,
                "activeParameter": 1
            }),
        ),
    )
}

fn handle_code_actions(
    writer: &mut impl io::Write,
    id: RequestId,
    params: Option<Value>,
) -> Result<()> {
    let params = params.context("code action params")?;
    let uri = params["textDocument"]["uri"]
        .as_str()
        .context("code action uri")?;
    let action_range = &params["range"];
    let line = action_range["start"]["line"].as_i64().unwrap_or(0);
    let character = action_range["start"]["character"].as_i64().unwrap_or(0);
    send_log(writer, &format!("code-action:{line}:{character}"))?;
    let result = json!([
        {
            "title": "Apply fake fix",
            "kind": "quickfix",
            "isPreferred": true,
            "edit": {
                "changes": {
                    uri: [{
                        "range": range(0, 0, 3),
                        "newText": "fixed"
                    }]
                }
            }
        },
        {
            "title": "Disabled fake fix",
            "disabled": {"reason": "not available"},
            "edit": {
                "changes": {
                    uri: [{
                        "range": range(0, 0, 3),
                        "newText": "disabled"
                    }]
                }
            }
        },
        {
            "title": "Resolve fake fix",
            "data": {"value": 1}
        },
        {
            "title": "Run fake command",
            "command": {
                "title": "Run fake command",
                "command": "fake.command",
                "arguments": [{"value": 1}]
            }
        }
    ]);
    send(writer, Message::response(id, result))
}

fn handle_formatting(
    writer: &mut impl io::Write,
    id: RequestId,
    method: &str,
    params: Option<Value>,
) -> Result<()> {
    let params = params.context("formatting params")?;
    let request_range = params
        .get("range")
        .map_or_else(|| "document".to_string(), Value::to_string);
    send_log(writer, &format!("format:{method}:{request_range}"))?;
    send(
        writer,
        Message::response(
            id,
            json!([{
                "range": range(0, 0, 3),
                "newText": "formatted"
            }]),
        ),
    )
}

fn handle_execute_command(
    writer: &mut impl io::Write,
    id: RequestId,
    params: Option<Value>,
) -> Result<()> {
    let params = params.context("execute command params")?;
    let command = params["command"].as_str().unwrap_or("");
    let arguments = params["arguments"].to_string();
    send_log(writer, &format!("execute:{command}:{arguments}"))?;
    send(writer, Message::response(id, Value::Null))
}

fn handle_workspace_diagnostic(
    writer: &mut impl io::Write,
    id: RequestId,
    params: Option<Value>,
    documents: &HashMap<String, i64>,
) -> Result<()> {
    let previous = params
        .as_ref()
        .and_then(|params| params.get("previousResultIds"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut items = Vec::new();
    for (uri, version) in documents {
        let result_id = format!("workspace:{version}");
        let unchanged = previous.iter().any(|previous| {
            previous.get("uri").and_then(Value::as_str) == Some(uri.as_str())
                && previous.get("value").and_then(Value::as_str) == Some(result_id.as_str())
        });
        if unchanged {
            items.push(json!({
                "kind": "unchanged",
                "uri": uri,
                "version": version,
                "resultId": result_id
            }));
        } else {
            items.push(json!({
                "kind": "full",
                "uri": uri,
                "version": version,
                "resultId": result_id,
                "items": [{
                    "range": {
                        "start": {"line": 0, "character": 0},
                        "end": {"line": 0, "character": 1}
                    },
                    "severity": 1,
                    "source": "workspace",
                    "code": "E0302",
                    "tags": [2],
                    "relatedInformation": [{
                        "location": {
                            "uri": uri,
                            "range": {
                                "start": {"line": 0, "character": 0},
                                "end": {"line": 0, "character": 1}
                            }
                        },
                        "message": "related workspace"
                    }],
                    "message": format!("workspace:{version}")
                }]
            }));
        }
    }
    send_log(writer, &format!("workspace-diagnostic:{}", previous.len()))?;
    send(writer, Message::response(id, json!({"items": items})))
}

fn handle_pull_diagnostic(
    writer: &mut impl io::Write,
    id: RequestId,
    params: Option<Value>,
    documents: &HashMap<String, i64>,
) -> Result<()> {
    let params = params.context("diagnostic params")?;
    let uri = params["textDocument"]["uri"]
        .as_str()
        .context("diagnostic uri")?;
    let previous = params.get("previousResultId").and_then(Value::as_str);
    let version = documents.get(uri).copied().unwrap_or(0);
    let result_id = format!("pull:{version}");
    send_log(
        writer,
        &format!("diagnostic:{version}:{}", previous.unwrap_or("none")),
    )?;
    if previous == Some(result_id.as_str()) {
        return send(
            writer,
            Message::response(id, json!({"kind": "unchanged", "resultId": result_id})),
        );
    }
    let result = json!({
        "kind": "full",
        "resultId": result_id,
        "items": [{
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 0, "character": 1}
            },
            "severity": 1,
            "source": "pull",
            "code": "E0301",
            "tags": [1],
            "relatedInformation": [{
                "location": {
                    "uri": uri,
                    "range": {
                        "start": {"line": 0, "character": 0},
                        "end": {"line": 0, "character": 1}
                    }
                },
                "message": "related"
            }],
            "message": format!("pull:{version}")
        }]
    });
    send(writer, Message::response(id, result))
}

fn handle_will_rename(
    writer: &mut impl io::Write,
    id: RequestId,
    params: Option<Value>,
) -> Result<()> {
    let old = file_uri_field(params.as_ref(), "oldUri");
    let new = file_uri_field(params.as_ref(), "newUri");
    send_log(writer, &format!("willRename:{old}:{new}"))?;
    let mut changes = serde_json::Map::new();
    changes.insert(
        old,
        json!([{
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 0, "character": 0}
            },
            "newText": ""
        }]),
    );
    send(
        writer,
        Message::response(id, json!({"changes": Value::Object(changes)})),
    )
}

fn file_uri_field(params: Option<&Value>, field: &str) -> String {
    params
        .and_then(|params| params.get("files"))
        .and_then(|files| files.as_array())
        .and_then(|files| files.first())
        .and_then(|file| file.get(field))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}
