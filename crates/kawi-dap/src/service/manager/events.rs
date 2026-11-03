use super::*;
use crate::client::ClientEvent;
use crate::protocol::Message;
use serde_json::Value;

mod incoming;
use std::path::Path;

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
                arguments,
            } => self.handle_reverse_request(seq, command, arguments),
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
        if success {
            self.handle_success(pending, body);
        } else {
            self.handle_failure(pending, command, message);
        }
    }

    fn handle_failure(
        &mut self,
        pending: PendingRequest,
        command: String,
        message: Option<String>,
    ) {
        let message = message.unwrap_or_else(|| format!("{command} failed"));
        match pending {
            PendingRequest::Initialize | PendingRequest::Launch => {
                self.finish_session(DebugStatus::Failed, Some(message));
            }
            PendingRequest::Disconnect => self.finish_session(DebugStatus::Terminated, None),
            PendingRequest::Continue { .. } | PendingRequest::Step => {
                self.emit(DapEvent::Error(message));
                self.emit(DapEvent::Status(DebugStatus::Stopped));
            }
            PendingRequest::Evaluate {
                expression,
                context,
            } => self.emit(DapEvent::EvaluationFailed {
                expression,
                context,
                message,
            }),
            _ => self.emit(DapEvent::Error(message)),
        }
    }

    fn handle_success(&mut self, pending: PendingRequest, body: Option<Value>) {
        match pending {
            PendingRequest::Initialize => self.handle_initialize_response(body.as_ref()),
            PendingRequest::Launch => {
                self.launch_sent = true;
                self.maybe_emit_running();
            }
            PendingRequest::SetBreakpoints { path } => {
                self.handle_breakpoints_response(&path, body.as_ref())
            }
            PendingRequest::ConfigurationDone => {
                self.configuration_done = true;
                self.maybe_emit_running();
            }
            PendingRequest::StackTrace { thread_id } => {
                self.handle_stack_response(thread_id, body.as_ref())
            }
            PendingRequest::Scopes { frame_id } => {
                self.handle_scopes_response(frame_id, body.as_ref())
            }
            PendingRequest::Variables {
                variables_reference,
                start,
            } => self.handle_variables_response(variables_reference, start, body.as_ref()),
            PendingRequest::SetVariable {
                variables_reference,
                name,
            } => self.handle_set_variable_response(variables_reference, name, body.as_ref()),
            PendingRequest::Restart | PendingRequest::Terminate => {}
            PendingRequest::Threads => self.handle_threads_response(body.as_ref()),
            PendingRequest::Evaluate {
                expression,
                context,
            } => self.handle_evaluate_response(expression, context, body.as_ref()),
            PendingRequest::SetFunctionBreakpoints | PendingRequest::SetExceptionBreakpoints => {}
            PendingRequest::Continue { thread_id } => self.handle_continue_response(thread_id),
            PendingRequest::Step => {}
            PendingRequest::Disconnect => self.finish_session(DebugStatus::Terminated, None),
        }
    }

    fn handle_initialize_response(&mut self, body: Option<&Value>) {
        self.initialize_deadline = None;
        self.available_exception_filters = super::parse::exception_filters(body);
        self.supports_function_breakpoints = body
            .and_then(|body| body.get("supportsFunctionBreakpoints"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        self.supports_restart_request = body
            .and_then(|body| body.get("supportsRestartRequest"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        self.supports_terminate_request = body
            .and_then(|body| body.get("supportsTerminateRequest"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        self.emit(DapEvent::Capabilities {
            supports_restart_request: self.supports_restart_request,
            supports_terminate_request: self.supports_terminate_request,
        });
        self.apply_default_exception_filters();
        self.emit(DapEvent::ExceptionFilters {
            filters: self.available_exception_filters.clone(),
        });
        self.send_launch();
    }

    fn apply_default_exception_filters(&mut self) {
        if self.exception_filters.is_none() {
            let defaults = self
                .available_exception_filters
                .iter()
                .filter(|filter| filter.default)
                .map(|filter| filter.filter.clone())
                .collect();
            self.exception_filters = Some(defaults);
            return;
        }
        if let Some(filters) = self.exception_filters.as_mut() {
            filters.retain(|filter| {
                self.available_exception_filters
                    .iter()
                    .any(|candidate| candidate.filter == *filter)
            });
        }
    }

    fn handle_breakpoints_response(&mut self, path: &Path, body: Option<&Value>) {
        for mut breakpoint in super::parse::breakpoints(body) {
            if breakpoint.source.is_none() {
                breakpoint.source = Some(path.to_path_buf());
            }
            self.emit(DapEvent::Breakpoint {
                reason: "changed".to_string(),
                breakpoint,
            });
        }
    }

    fn handle_stack_response(&mut self, thread_id: i64, body: Option<&Value>) {
        self.emit(DapEvent::StackTrace {
            thread_id,
            frames: super::parse::stack_frames(body),
        });
    }

    fn handle_scopes_response(&mut self, frame_id: i64, body: Option<&Value>) {
        self.emit(DapEvent::Scopes {
            frame_id,
            scopes: super::parse::scopes(body),
        });
    }

    fn handle_variables_response(
        &mut self,
        variables_reference: i64,
        start: i64,
        body: Option<&Value>,
    ) {
        self.emit(DapEvent::Variables {
            variables_reference,
            variables: super::parse::variables(body),
            start,
        });
    }

    fn handle_set_variable_response(
        &mut self,
        variables_reference: i64,
        name: String,
        body: Option<&Value>,
    ) {
        let (value, kind, new_reference) = super::parse::set_variable_result(body);
        self.emit(DapEvent::VariableSet {
            variables_reference,
            name,
            value,
            kind,
            new_reference,
        });
    }

    fn handle_threads_response(&mut self, body: Option<&Value>) {
        self.emit(DapEvent::Threads {
            threads: super::parse::threads(body),
        });
    }

    fn handle_evaluate_response(
        &mut self,
        expression: String,
        context: String,
        body: Option<&Value>,
    ) {
        let (result, kind, variables_reference) = super::parse::evaluate_result(body);
        self.emit(DapEvent::Evaluated {
            expression,
            context,
            result,
            kind,
            variables_reference,
        });
    }

    fn handle_continue_response(&mut self, thread_id: i64) {
        self.emit(DapEvent::Status(DebugStatus::Running));
        self.emit(DapEvent::Continued {
            thread_id,
            all_threads_continued: false,
        });
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
