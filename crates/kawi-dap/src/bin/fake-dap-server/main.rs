mod server;

use anyhow::Result;
use kawi_dap::Message;
use kawi_jsonrpc::framing;
use serde_json::{json, Value};
use server::Server;
use std::io::{self, BufReader};

fn main() -> Result<()> {
    let mut reader = BufReader::new(io::stdin());
    let mut writer = io::stdout();
    let mut server = Server::new();
    while let Some(value) = framing::read_message(&mut reader)? {
        let message = Message::parse(value)?;
        if let Message::Response { success, body, .. } = &message {
            if let Some(path) = server.terminal_file.as_ref() {
                let response = json!({"success": success, "body": body});
                std::fs::write(path, serde_json::to_vec(&response)?)?;
            }
            continue;
        }
        let Message::Request {
            seq,
            command,
            arguments,
        } = message
        else {
            continue;
        };
        if let Some(path) = server.request_log.as_ref() {
            let mut log = std::fs::read_to_string(path).unwrap_or_default();
            log.push_str(&command);
            if let Some(arguments) = arguments.as_ref() {
                log.push(' ');
                log.push_str(&serde_json::to_string(arguments)?);
            }
            log.push('\n');
            std::fs::write(path, log)?;
        }
        match command.as_str() {
            "initialize" => {
                server.response(
                    &mut writer,
                    seq,
                    &command,
                    Some(json!({
                        "supportsConfigurationDoneRequest": true,
                        "supportsFunctionBreakpoints": true,
                        "supportsConditionalBreakpoints": true,
                        "supportsHitConditionalBreakpoints": true,
                        "supportsEvaluateForHovers": false,
                        "supportsStepBack": false,
                        "supportsSetVariable": true,
                        "supportsRestartFrame": false,
                        "supportsGotoTargetsRequest": false,
                        "supportsCompletionsRequest": false,
                        "supportsModulesRequest": false,
                        "supportsRestartRequest": true,
                        "supportsExceptionOptions": false,
                        "supportsValueFormattingOptions": false,
                        "supportsExceptionInfoRequest": false,
                        "supportsTerminateRequest": true,
                        "supportsDataBreakpoints": false,
                        "supportsReadMemoryRequest": false,
                        "supportsWriteMemoryRequest": false,
                        "supportsDisassembleRequest": false,
                        "supportsSteppingGranularity": false,
                        "supportsInstructionBreakpoints": false,
                        "exceptionBreakpointFilters": [
                            {"filter": "uncaught", "label": "Uncaught Exceptions", "default": true},
                            {"filter": "caught", "label": "Caught Exceptions", "default": false}
                        ],
                    })),
                )?;
            }
            "launch" | "attach" => {
                server.response(&mut writer, seq, &command, None)?;
                server.event(&mut writer, "initialized", None)?;
                server.event(
                    &mut writer,
                    "output",
                    Some(json!({"category": "console", "output": "started\n"})),
                )?;
            }
            "setBreakpoints" => {
                let lines = arguments
                    .as_ref()
                    .and_then(|arguments| arguments.get("breakpoints"))
                    .and_then(Value::as_array)
                    .map(|breakpoints| {
                        breakpoints
                            .iter()
                            .filter_map(|breakpoint| breakpoint.get("line").and_then(Value::as_i64))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                server.breakpoints = lines.clone();
                let breakpoints: Vec<Value> = lines
                    .iter()
                    .map(|line| json!({"verified": true, "line": line}))
                    .collect();
                server.response(
                    &mut writer,
                    seq,
                    &command,
                    Some(json!({"breakpoints": breakpoints})),
                )?;
            }
            "configurationDone" => {
                server.response(&mut writer, seq, &command, None)?;
                if server.run_in_terminal {
                    let request_seq = server.next_seq();
                    server.send(
                        &mut writer,
                        Message::request(
                            request_seq,
                            "runInTerminal",
                            Some(json!({
                                "kind": "integrated",
                                "cwd": std::env::current_dir()?.display().to_string(),
                                "args": ["printf", "debug-terminal"],
                                "env": {"KAWI_DEBUG_TEST": "1"},
                            })),
                        ),
                    )?;
                }
                if server.stop_on_entry {
                    server.stopped(&mut writer, "entry")?;
                }
            }
            "continue" => {
                server.response(
                    &mut writer,
                    seq,
                    &command,
                    Some(json!({"allThreadsContinued": true})),
                )?;
                server.event(
                    &mut writer,
                    "continued",
                    Some(json!({"threadId": 1, "allThreadsContinued": true})),
                )?;
                server.stopped(&mut writer, "breakpoint")?;
            }
            "next" | "stepIn" | "stepOut" => {
                server.response(&mut writer, seq, &command, None)?;
                server.stopped(&mut writer, "step")?;
            }
            "stackTrace" => {
                let source = server.source.display().to_string();
                server.response(
                    &mut writer,
                    seq,
                    &command,
                    Some(json!({
                        "stackFrames": [
                            {
                                "id": 1,
                                "name": "main",
                                "source": {"path": source},
                                "line": 2,
                                "column": 1,
                            },
                            {
                                "id": 2,
                                "name": "helper",
                                "source": {"path": source},
                                "line": 5,
                                "column": 3,
                            }
                        ],
                        "totalFrames": 2,
                    })),
                )?;
            }
            "scopes" => {
                server.response(
                    &mut writer,
                    seq,
                    &command,
                    Some(json!({
                        "scopes": [
                            {"name": "Locals", "variablesReference": 100, "expensive": false}
                        ]
                    })),
                )?;
            }
            "variables" => {
                let start = arguments
                    .as_ref()
                    .and_then(|arguments| arguments.get("start"))
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .max(0) as usize;
                let count = arguments
                    .as_ref()
                    .and_then(|arguments| arguments.get("count"))
                    .and_then(Value::as_i64)
                    .unwrap_or(4)
                    .max(0) as usize;
                let variables = [
                    json!({"name": "x", "value": "42", "type": "int", "variablesReference": 0}),
                    json!({"name": "s", "value": "hello", "type": "str", "variablesReference": 0}),
                    json!({"name": "y", "value": "7", "type": "int", "variablesReference": 0}),
                    json!({"name": "z", "value": "9", "type": "int", "variablesReference": 0}),
                ];
                let page: Vec<Value> = variables.into_iter().skip(start).take(count).collect();
                server.response(&mut writer, seq, &command, Some(json!({"variables": page})))?;
            }
            "setVariable" => {
                let value = arguments
                    .as_ref()
                    .and_then(|arguments| arguments.get("value"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                server.response(
                    &mut writer,
                    seq,
                    &command,
                    Some(json!({
                        "value": value,
                        "type": "string",
                        "variablesReference": 0,
                    })),
                )?;
            }
            "restart" => {
                server.response(&mut writer, seq, &command, None)?;
                server.stopped(&mut writer, "restart")?;
            }
            "threads" => {
                server.response(
                    &mut writer,
                    seq,
                    &command,
                    Some(json!({
                        "threads": [
                            {"id": 1, "name": "main"},
                            {"id": 2, "name": "worker"}
                        ]
                    })),
                )?;
            }
            "evaluate" => {
                let expression = arguments
                    .as_ref()
                    .and_then(|arguments| arguments.get("expression"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let context = arguments
                    .as_ref()
                    .and_then(|arguments| arguments.get("context"))
                    .and_then(Value::as_str)
                    .unwrap_or("repl");
                if expression == "missing" {
                    server.failure(&mut writer, seq, &command, "unknown expression")?;
                } else {
                    server.response(
                        &mut writer,
                        seq,
                        &command,
                        Some(json!({
                            "result": format!("{expression}:{context}"),
                            "type": "string",
                            "variablesReference": 0,
                        })),
                    )?;
                }
            }
            "setFunctionBreakpoints" | "setExceptionBreakpoints" => {
                server.response(&mut writer, seq, &command, Some(json!({"breakpoints": []})))?;
            }
            "disconnect" | "terminate" => {
                server.response(&mut writer, seq, &command, None)?;
                server.event(&mut writer, "terminated", None)?;
                return Ok(());
            }
            other => server.failure(&mut writer, seq, other, "unsupported request")?,
        }
    }
    Ok(())
}
