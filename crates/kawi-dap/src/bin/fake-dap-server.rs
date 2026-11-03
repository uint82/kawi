use anyhow::Result;
use kawi_dap::Message;
use kawi_jsonrpc::framing;
use serde_json::{json, Value};
use std::io::{self, BufReader};
use std::path::PathBuf;

struct Server {
    seq: i64,
    source: PathBuf,
    stop_on_entry: bool,
    breakpoints: Vec<i64>,
}

impl Server {
    fn new() -> Self {
        let source = std::env::var("FAKE_DAP_SOURCE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp/fake/main.rs"));
        let stop_on_entry = std::env::var("FAKE_DAP_STOP_ON_ENTRY").is_ok_and(|value| value == "1");
        if let Ok(path) = std::env::var("FAKE_DAP_PID_FILE") {
            let _ = std::fs::write(PathBuf::from(path), std::process::id().to_string());
        }
        Self {
            seq: 0,
            source,
            stop_on_entry,
            breakpoints: Vec::new(),
        }
    }

    fn next_seq(&mut self) -> i64 {
        self.seq = self.seq.wrapping_add(1);
        self.seq
    }

    fn send(&mut self, writer: &mut impl std::io::Write, message: Message) -> Result<()> {
        framing::write_message(writer, &message.to_value())
    }

    fn response(
        &mut self,
        writer: &mut impl std::io::Write,
        request_seq: i64,
        command: &str,
        body: Option<Value>,
    ) -> Result<()> {
        let seq = self.next_seq();
        self.send(
            writer,
            Message::response(seq, request_seq, true, command, None, body),
        )
    }

    fn failure(
        &mut self,
        writer: &mut impl std::io::Write,
        request_seq: i64,
        command: &str,
        message: &str,
    ) -> Result<()> {
        let seq = self.next_seq();
        self.send(
            writer,
            Message::response(
                seq,
                request_seq,
                false,
                command,
                Some(message.to_string()),
                None,
            ),
        )
    }

    fn event(
        &mut self,
        writer: &mut impl std::io::Write,
        event: &str,
        body: Option<Value>,
    ) -> Result<()> {
        let seq = self.next_seq();
        self.send(writer, Message::event(seq, event, body))
    }

    fn stopped(&mut self, writer: &mut impl std::io::Write, reason: &str) -> Result<()> {
        self.event(
            writer,
            "stopped",
            Some(json!({
                "reason": reason,
                "threadId": 1,
                "allThreadsStopped": true,
            })),
        )
    }
}

fn main() -> Result<()> {
    let mut reader = BufReader::new(io::stdin());
    let mut writer = io::stdout();
    let mut server = Server::new();
    while let Some(value) = framing::read_message(&mut reader)? {
        let message = Message::parse(value)?;
        let Message::Request {
            seq,
            command,
            arguments,
        } = message
        else {
            continue;
        };
        match command.as_str() {
            "initialize" => {
                server.response(
                    &mut writer,
                    seq,
                    &command,
                    Some(json!({
                        "supportsConfigurationDoneRequest": true,
                        "supportsFunctionBreakpoints": false,
                        "supportsConditionalBreakpoints": true,
                        "supportsHitConditionalBreakpoints": true,
                        "supportsEvaluateForHovers": false,
                        "supportsStepBack": false,
                        "supportsSetVariable": false,
                        "supportsRestartFrame": false,
                        "supportsGotoTargetsRequest": false,
                        "supportsCompletionsRequest": false,
                        "supportsModulesRequest": false,
                        "supportsRestartRequest": false,
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
                server.response(
                    &mut writer,
                    seq,
                    &command,
                    Some(json!({
                        "variables": [
                            {"name": "x", "value": "42", "type": "int", "variablesReference": 0},
                            {"name": "s", "value": "hello", "type": "str", "variablesReference": 0}
                        ]
                    })),
                )?;
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
