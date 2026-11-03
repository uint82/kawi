use super::*;
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
            Command::Start {
                config,
                breakpoints,
            } => self.start_session(config, breakpoints),
            Command::Stop => self.stop_session(),
            Command::SetBreakpoints { path, breakpoints } => {
                self.breakpoints.insert(path.clone(), breakpoints.clone());
                if self.initialized {
                    self.send_breakpoints(&path, &breakpoints);
                }
            }
            Command::Continue { thread_id } => {
                self.send_request(
                    "continue",
                    Some(serde_json::json!({"threadId": thread_id})),
                    PendingRequest::Continue { thread_id },
                );
            }
            Command::Step { thread_id, kind } => {
                self.send_request(
                    kind.command(),
                    Some(serde_json::json!({"threadId": thread_id})),
                    PendingRequest::Step,
                );
            }
            Command::StackTrace { thread_id } => {
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
            Command::Scopes { frame_id } => {
                self.send_request(
                    "scopes",
                    Some(serde_json::json!({"frameId": frame_id})),
                    PendingRequest::Scopes { frame_id },
                );
            }
            Command::Variables {
                variables_reference,
            } => {
                self.send_request(
                    "variables",
                    Some(serde_json::json!({"variablesReference": variables_reference})),
                    PendingRequest::Variables {
                        variables_reference,
                    },
                );
            }
            Command::Shutdown => {}
        }
    }

    fn start_session(&mut self, config: ExpandedConfiguration, breakpoints: Vec<BreakpointSet>) {
        if self.client.is_some() {
            self.finish_session(DebugStatus::Terminated, None);
        }
        self.breakpoints = breakpoints
            .into_iter()
            .map(|set| (set.path, set.breakpoints))
            .collect();
        self.pending.clear();
        self.initialized = false;
        self.launch_sent = false;
        self.configuration_done = false;
        self.stopping = false;
        self.next_generation = self.next_generation.wrapping_add(1);
        let generation = self.next_generation;
        let client = Client::spawn(
            generation,
            &config.command,
            &config.args,
            &config.env,
            config.cwd.as_deref(),
            self.input_tx.clone(),
        );
        let client = match client {
            Ok(client) => client,
            Err(error) => {
                self.config = Some(config);
                self.emit(DapEvent::Error(error.to_string()));
                self.emit(DapEvent::Status(DebugStatus::Failed));
                return;
            }
        };
        self.client = Some(ClientState { client, generation });
        self.config = Some(config);
        self.emit(DapEvent::Status(DebugStatus::Starting));
        self.send_initialize();
    }

    fn send_initialize(&mut self) {
        let Some(config) = self.config.as_ref() else {
            return;
        };
        let arguments = serde_json::json!({
            "clientID": "kawi",
            "clientName": "Kawi",
            "adapterID": config.adapter,
            "pathFormat": "path",
            "linesStartAt1": true,
            "columnsStartAt1": true,
            "supportsVariableType": true,
            "supportsVariablePaging": false,
            "supportsRunInTerminalRequest": false,
            "supportsMemoryReferences": false,
            "supportsProgressReporting": false,
        });
        if self.send_request("initialize", Some(arguments), PendingRequest::Initialize) {
            self.initialize_deadline = Some(Instant::now() + INITIALIZE_TIMEOUT);
        } else {
            self.finish_session(
                DebugStatus::Failed,
                Some("debug adapter is not accepting requests".to_string()),
            );
        }
    }

    fn stop_session(&mut self) {
        if self.client.is_none() {
            self.emit(DapEvent::Terminated);
            self.emit(DapEvent::Status(DebugStatus::Terminated));
            return;
        }
        self.stopping = true;
        let sent = self.send_request(
            "disconnect",
            Some(serde_json::json!({
                "restart": false,
                "terminateDebuggee": true,
            })),
            PendingRequest::Disconnect,
        );
        if sent {
            self.disconnect_deadline = Some(Instant::now() + DISCONNECT_TIMEOUT);
        } else {
            self.finish_session(DebugStatus::Terminated, None);
        }
    }

    fn shutdown(&mut self) {
        if let Some(mut client) = self.client.take() {
            client.client.terminate();
        }
        self.pending.clear();
    }

    pub(super) fn finish_session(&mut self, status: DebugStatus, message: Option<String>) {
        if let Some(mut client) = self.client.take() {
            client.client.terminate();
        }
        self.pending.clear();
        self.initialized = false;
        self.launch_sent = false;
        self.configuration_done = false;
        self.initialize_deadline = None;
        self.disconnect_deadline = None;
        self.stopping = false;
        self.emit(DapEvent::Terminated);
        if let Some(message) = message {
            self.emit(DapEvent::Error(message));
        }
        self.emit(DapEvent::Status(status));
    }

    pub(super) fn is_current_client(&self, generation: u64) -> bool {
        self.client
            .as_ref()
            .is_some_and(|client| client.generation == generation)
    }
}
