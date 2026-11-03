use super::*;
use std::path::PathBuf;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Instant;

enum InputWait {
    Input(Box<ServiceInput>),
    Timeout,
    Closed,
}

impl Manager {
    pub(crate) fn run(mut self) {
        loop {
            match self.next_input() {
                InputWait::Input(input) => match *input {
                    ServiceInput::Command(Command::Shutdown) => {
                        self.shutdown();
                        return;
                    }
                    ServiceInput::Command(command) => {
                        self.handle_command(command);
                        self.expire_deadlines();
                    }
                    ServiceInput::Client(event) => {
                        self.handle_client_event(event);
                        self.expire_deadlines();
                    }
                },
                InputWait::Timeout => self.expire_deadlines(),
                InputWait::Closed => {
                    self.shutdown();
                    return;
                }
            }
        }
    }

    fn next_input(&self) -> InputWait {
        let Some(timeout) = self.next_deadline() else {
            return self
                .input
                .recv()
                .map_or(InputWait::Closed, |input| InputWait::Input(Box::new(input)));
        };
        match self.input.recv_timeout(timeout) {
            Ok(input) => InputWait::Input(Box::new(input)),
            Err(RecvTimeoutError::Timeout) => InputWait::Timeout,
            Err(RecvTimeoutError::Disconnected) => InputWait::Closed,
        }
    }

    fn next_deadline(&self) -> Option<Duration> {
        [self.initialize_deadline, self.disconnect_deadline]
            .into_iter()
            .flatten()
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .min()
    }

    fn expire_deadlines(&mut self) {
        if self
            .initialize_deadline
            .is_some_and(|deadline| deadline <= Instant::now())
        {
            self.finish_session(
                DebugStatus::Failed,
                Some("debug adapter initialize timed out".to_string()),
            );
        }
        if self
            .disconnect_deadline
            .is_some_and(|deadline| deadline <= Instant::now())
        {
            self.finish_session(DebugStatus::Terminated, None);
        }
    }

    fn handle_command(&mut self, command: Command) {
        match command {
            Command::Start(request) => self.start_session(*request),
            Command::Stop => self.stop_session(),
            Command::SetBreakpoints { path, breakpoints } => {
                self.set_breakpoints(path, breakpoints)
            }
            Command::Continue { thread_id } => self.continue_thread(thread_id),
            Command::Step { thread_id, kind } => self.step_thread(thread_id, kind),
            Command::StackTrace { thread_id } => self.stack_trace(thread_id),
            Command::Scopes { frame_id } => self.scopes(frame_id),
            Command::Variables {
                variables_reference,
                start,
                count,
            } => self.variables(variables_reference, start, count),
            Command::SetVariable {
                variables_reference,
                name,
                value,
            } => self.set_variable(variables_reference, name, value),
            Command::Restart => self.restart(),
            Command::Terminate => self.terminate(),
            Command::Threads => self.threads(),
            Command::Evaluate {
                expression,
                frame_id,
                context,
            } => self.evaluate(expression, frame_id, context),
            Command::SetFunctionBreakpoints { breakpoints } => {
                self.set_function_breakpoints(breakpoints)
            }
            Command::SetExceptionFilters { filters } => self.set_exception_filters(filters),
            Command::RunInTerminalResponse {
                request_seq,
                process_id,
                message,
            } => self.respond_run_in_terminal(request_seq, process_id, message),
            Command::Shutdown => {}
        }
    }

    fn set_breakpoints(&mut self, path: PathBuf, breakpoints: Vec<SourceBreakpoint>) {
        self.breakpoints.insert(path.clone(), breakpoints.clone());
        if self.initialized {
            self.send_breakpoints(&path, &breakpoints);
        }
    }

    fn continue_thread(&mut self, thread_id: i64) {
        self.send_request(
            "continue",
            Some(serde_json::json!({"threadId": thread_id})),
            PendingRequest::Continue { thread_id },
        );
    }

    fn step_thread(&mut self, thread_id: i64, kind: StepKind) {
        self.send_request(
            kind.command(),
            Some(serde_json::json!({"threadId": thread_id})),
            PendingRequest::Step,
        );
    }

    fn stack_trace(&mut self, thread_id: i64) {
        self.send_request(
            "stackTrace",
            Some(serde_json::json!({
                "threadId": thread_id,
                "startFrame": 0,
                "levels": STACK_FRAME_LIMIT,
            })),
            PendingRequest::StackTrace { thread_id },
        );
    }

    fn scopes(&mut self, frame_id: i64) {
        self.send_request(
            "scopes",
            Some(serde_json::json!({"frameId": frame_id})),
            PendingRequest::Scopes { frame_id },
        );
    }

    fn variables(&mut self, variables_reference: i64, start: i64, count: i64) {
        self.send_request(
            "variables",
            Some(serde_json::json!({
                "variablesReference": variables_reference,
                "start": start,
                "count": count,
            })),
            PendingRequest::Variables {
                variables_reference,
                start,
            },
        );
    }

    fn set_variable(&mut self, variables_reference: i64, name: String, value: String) {
        self.send_request(
            "setVariable",
            Some(serde_json::json!({
                "variablesReference": variables_reference,
                "name": name,
                "value": value,
            })),
            PendingRequest::SetVariable {
                variables_reference,
                name,
            },
        );
    }

    fn restart(&mut self) {
        if self.send_request("restart", None, PendingRequest::Restart) {
            self.emit(DapEvent::Log("debug adapter restart requested".to_string()));
        }
    }

    fn terminate(&mut self) {
        if self.send_request("terminate", None, PendingRequest::Terminate) {
            self.emit(DapEvent::Log(
                "debug adapter terminate requested".to_string(),
            ));
        }
    }

    fn threads(&mut self) {
        self.send_request("threads", None, PendingRequest::Threads);
    }

    fn evaluate(&mut self, expression: String, frame_id: Option<i64>, context: String) {
        let mut arguments = serde_json::json!({
            "expression": expression,
            "context": context,
        });
        if let Some(frame_id) = frame_id {
            arguments["frameId"] = serde_json::json!(frame_id);
        }
        self.send_request(
            "evaluate",
            Some(arguments),
            PendingRequest::Evaluate {
                expression,
                context,
            },
        );
    }

    fn set_function_breakpoints(&mut self, breakpoints: Vec<FunctionBreakpoint>) {
        self.function_breakpoints = breakpoints;
        if self.initialized {
            self.send_function_breakpoints();
        }
    }

    fn set_exception_filters(&mut self, filters: Vec<String>) {
        self.exception_filters = Some(filters);
        if self.initialized {
            self.send_exception_breakpoints();
        }
    }
}
