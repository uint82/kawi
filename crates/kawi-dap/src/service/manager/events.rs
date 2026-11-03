use super::*;
use crate::client::ClientEvent;
use crate::protocol::Message;
use serde_json::Value;

impl Manager {
    pub(super) fn handle_client_event(&mut self, event: ClientEvent) {
        match event {
            ClientEvent::Message {
                generation,
                message,
            } => {
                if self.is_current_client(generation) {
                    self.handle_message(message);
                }
            }
            ClientEvent::ReaderClosed { generation, error } => {
                if self.is_current_client(generation) {
                    let message =
                        error.unwrap_or_else(|| "debug adapter closed stdout".to_string());
                    self.client_failed(generation, message);
                }
            }
            ClientEvent::WriterClosed { generation, error } => {
                if self.is_current_client(generation) {
                    self.client_failed(generation, error);
                }
            }
            ClientEvent::Stderr { generation, line } => {
                if self.is_current_client(generation) {
                    self.emit(DapEvent::Log(line));
                }
            }
        }
    }

    fn handle_message(&mut self, message: Message) {
        match message {
            Message::Request {
                seq,
                command,
                arguments: _,
            } => self.handle_reverse_request(seq, command),
            Message::Response {
                request_seq,
                success,
                command,
                message,
                body,
                ..
            } => self.handle_response(request_seq, success, command, message, body),
            Message::Event { event, body, .. } => self.handle_event(&event, body.as_ref()),
        }
    }

    fn handle_response(
        &mut self,
        request_seq: i64,
        success: bool,
        command: String,
        message: Option<String>,
        body: Option<Value>,
    ) {
        let Some(pending) = self.pending.remove(&request_seq) else {
            return;
        };
        if !success {
            let message = message.unwrap_or_else(|| format!("{command} failed"));
            if matches!(pending, PendingRequest::Initialize | PendingRequest::Launch) {
                self.finish_session(DebugStatus::Failed, Some(message));
            } else if matches!(pending, PendingRequest::Disconnect) {
                self.finish_session(DebugStatus::Terminated, None);
            } else if matches!(
                pending,
                PendingRequest::Continue { .. } | PendingRequest::Step
            ) {
                self.emit(DapEvent::Error(message));
                self.emit(DapEvent::Status(DebugStatus::Stopped));
            } else {
                self.emit(DapEvent::Error(message));
            }
            return;
        }
        match pending {
            PendingRequest::Initialize => {
                self.initialize_deadline = None;
                self.send_launch();
            }
            PendingRequest::Launch => {
                self.launch_sent = true;
                self.maybe_emit_running();
            }
            PendingRequest::SetBreakpoints { path } => {
                for mut breakpoint in super::parse::breakpoints(body.as_ref()) {
                    if breakpoint.source.is_none() {
                        breakpoint.source = Some(path.clone());
                    }
                    self.emit(DapEvent::Breakpoint {
                        reason: "changed".to_string(),
                        breakpoint,
                    });
                }
            }
            PendingRequest::ConfigurationDone => {
                self.configuration_done = true;
                self.maybe_emit_running();
            }
            PendingRequest::StackTrace { thread_id } => {
                self.emit(DapEvent::StackTrace {
                    thread_id,
                    frames: super::parse::stack_frames(body.as_ref()),
                });
            }
            PendingRequest::Scopes { frame_id } => {
                self.emit(DapEvent::Scopes {
                    frame_id,
                    scopes: super::parse::scopes(body.as_ref()),
                });
            }
            PendingRequest::Variables {
                variables_reference,
            } => {
                self.emit(DapEvent::Variables {
                    variables_reference,
                    variables: super::parse::variables(body.as_ref()),
                });
            }
            PendingRequest::Continue { thread_id } => {
                self.emit(DapEvent::Status(DebugStatus::Running));
                self.emit(DapEvent::Continued {
                    thread_id,
                    all_threads_continued: false,
                });
            }
            PendingRequest::Step => {}
            PendingRequest::Disconnect => {
                self.finish_session(DebugStatus::Terminated, None);
            }
        }
    }

    fn handle_event(&mut self, event: &str, body: Option<&Value>) {
        match event {
            "initialized" => {
                if self.stopping {
                    return;
                }
                self.initialized = true;
                self.send_launch();
                self.send_all_breakpoints();
                self.send_configuration_done();
            }
            "stopped" => {
                self.emit(DapEvent::Status(DebugStatus::Stopped));
                self.emit(DapEvent::Stopped {
                    reason: string_field(body, "reason").unwrap_or_else(|| "pause".to_string()),
                    thread_id: body
                        .and_then(|body| body.get("threadId"))
                        .and_then(Value::as_i64),
                    all_threads_stopped: body
                        .and_then(|body| body.get("allThreadsStopped"))
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    description: string_field(body, "description"),
                    text: string_field(body, "text"),
                });
            }
            "continued" => {
                let thread_id = body
                    .and_then(|body| body.get("threadId"))
                    .and_then(Value::as_i64)
                    .unwrap_or(0);
                let all_threads_continued = body
                    .and_then(|body| body.get("allThreadsContinued"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                self.emit(DapEvent::Status(DebugStatus::Running));
                self.emit(DapEvent::Continued {
                    thread_id,
                    all_threads_continued,
                });
            }
            "exited" => {
                self.emit(DapEvent::Exited {
                    exit_code: body
                        .and_then(|body| body.get("exitCode"))
                        .and_then(Value::as_i64)
                        .unwrap_or(0),
                });
            }
            "terminated" => self.finish_session(DebugStatus::Terminated, None),
            "output" => self.emit(DapEvent::Output {
                category: string_field(body, "category").unwrap_or_else(|| "console".to_string()),
                output: string_field(body, "output").unwrap_or_default(),
            }),
            "breakpoint" => {
                let breakpoint = body
                    .and_then(|body| body.get("breakpoint"))
                    .map(super::parse::breakpoint);
                if let Some(breakpoint) = breakpoint {
                    self.emit(DapEvent::Breakpoint {
                        reason: string_field(body, "reason")
                            .unwrap_or_else(|| "changed".to_string()),
                        breakpoint,
                    });
                }
            }
            _ => {}
        }
    }

    fn handle_reverse_request(&mut self, request_seq: i64, command: String) {
        let seq = self.request_id();
        if let Some(client) = self.client.as_ref() {
            client.client.send(Message::response(
                seq,
                request_seq,
                false,
                command,
                Some("unsupported request".to_string()),
                None,
            ));
        }
    }

    fn client_failed(&mut self, generation: u64, message: String) {
        if !self.is_current_client(generation) {
            return;
        }
        let status = if self.stopping {
            DebugStatus::Terminated
        } else {
            DebugStatus::Failed
        };
        let message = (!self.stopping).then_some(message);
        self.finish_session(status, message);
    }
}

fn string_field(body: Option<&Value>, key: &str) -> Option<String> {
    body.and_then(|body| body.get(key))
        .and_then(Value::as_str)
        .map(str::to_string)
}
