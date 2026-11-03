use super::*;
use std::time::Instant;

impl Manager {
    pub(super) fn start_session(&mut self, request: StartRequest) {
        if self.client.is_some() {
            self.finish_session(DebugStatus::Terminated, None);
        }
        let StartRequest {
            config,
            breakpoints,
            function_breakpoints,
            exception_filters,
        } = request;
        self.breakpoints = breakpoints
            .into_iter()
            .map(|set| (set.path, set.breakpoints))
            .collect();
        self.function_breakpoints = function_breakpoints;
        self.exception_filters = exception_filters;
        self.available_exception_filters.clear();
        self.supports_function_breakpoints = false;
        self.reverse_requests.clear();
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
            "supportsRunInTerminalRequest": true,
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

    pub(super) fn stop_session(&mut self) {
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

    pub(super) fn shutdown(&mut self) {
        if let Some(mut client) = self.client.take() {
            client.client.terminate();
        }
        self.pending.clear();
        self.reverse_requests.clear();
    }

    pub(super) fn finish_session(&mut self, status: DebugStatus, message: Option<String>) {
        if let Some(mut client) = self.client.take() {
            client.client.terminate();
        }
        self.pending.clear();
        self.reverse_requests.clear();
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
