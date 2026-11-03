use anyhow::Result;
use kawi_dap::Message;
use kawi_jsonrpc::framing;
use serde_json::{json, Value};
use std::path::PathBuf;

pub(crate) struct Server {
    pub(crate) seq: i64,
    pub(crate) source: PathBuf,
    pub(crate) stop_on_entry: bool,
    pub(crate) run_in_terminal: bool,
    pub(crate) terminal_file: Option<PathBuf>,
    pub(crate) request_log: Option<PathBuf>,
    pub(crate) breakpoints: Vec<i64>,
}

impl Server {
    pub(crate) fn new() -> Self {
        let source = std::env::var("FAKE_DAP_SOURCE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp/fake/main.rs"));
        let stop_on_entry = std::env::var("FAKE_DAP_STOP_ON_ENTRY").is_ok_and(|value| value == "1");
        let run_in_terminal =
            std::env::var("FAKE_DAP_RUN_IN_TERMINAL").is_ok_and(|value| value == "1");
        let terminal_file = std::env::var("FAKE_DAP_TERMINAL_FILE")
            .ok()
            .map(PathBuf::from);
        let request_log = std::env::var("FAKE_DAP_REQUEST_LOG")
            .ok()
            .map(PathBuf::from);
        if let Ok(path) = std::env::var("FAKE_DAP_PID_FILE") {
            let _ = std::fs::write(PathBuf::from(path), std::process::id().to_string());
        }
        Self {
            seq: 0,
            source,
            stop_on_entry,
            run_in_terminal,
            terminal_file,
            request_log,
            breakpoints: Vec::new(),
        }
    }

    pub(crate) fn next_seq(&mut self) -> i64 {
        self.seq = self.seq.wrapping_add(1);
        self.seq
    }

    pub(crate) fn send(
        &mut self,
        writer: &mut impl std::io::Write,
        message: Message,
    ) -> Result<()> {
        framing::write_message(writer, &message.to_value())
    }

    pub(crate) fn response(
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

    pub(crate) fn failure(
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

    pub(crate) fn event(
        &mut self,
        writer: &mut impl std::io::Write,
        event: &str,
        body: Option<Value>,
    ) -> Result<()> {
        let seq = self.next_seq();
        self.send(writer, Message::event(seq, event, body))
    }

    pub(crate) fn stopped(&mut self, writer: &mut impl std::io::Write, reason: &str) -> Result<()> {
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
