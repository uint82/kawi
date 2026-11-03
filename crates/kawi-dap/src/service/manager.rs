use super::*;
use crate::ExpandedConfiguration;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

mod events;
mod launch;
mod parse;
mod reverse;
mod run;
mod session;

pub(crate) struct Manager {
    input: Receiver<ServiceInput>,
    input_tx: Sender<ServiceInput>,
    events: Sender<DapEvent>,
    wake: Sender<()>,
    client: Option<ClientState>,
    config: Option<ExpandedConfiguration>,
    breakpoints: BTreeMap<PathBuf, Vec<SourceBreakpoint>>,
    function_breakpoints: Vec<FunctionBreakpoint>,
    exception_filters: Option<Vec<String>>,
    available_exception_filters: Vec<ExceptionFilter>,
    supports_function_breakpoints: bool,
    supports_restart_request: bool,
    supports_terminate_request: bool,
    reverse_requests: BTreeMap<i64, String>,
    pending: std::collections::HashMap<i64, PendingRequest>,
    next_generation: u64,
    next_seq: i64,
    initialized: bool,
    launch_sent: bool,
    configuration_done: bool,
    initialize_deadline: Option<std::time::Instant>,
    disconnect_deadline: Option<std::time::Instant>,
    stopping: bool,
}

pub(crate) struct ClientState {
    pub(crate) client: Client,
    pub(crate) generation: u64,
}

pub(crate) enum PendingRequest {
    Initialize,
    Launch,
    SetBreakpoints {
        path: PathBuf,
    },
    ConfigurationDone,
    StackTrace {
        thread_id: i64,
    },
    Scopes {
        frame_id: i64,
    },
    Variables {
        variables_reference: i64,
        start: i64,
    },
    SetVariable {
        variables_reference: i64,
        name: String,
    },
    Restart,
    Terminate,
    Threads,
    Evaluate {
        expression: String,
        context: String,
    },
    SetFunctionBreakpoints,
    SetExceptionBreakpoints,
    Continue {
        thread_id: i64,
    },
    Step,
    Disconnect,
}

impl Manager {
    pub(super) fn new(
        input: Receiver<ServiceInput>,
        input_tx: Sender<ServiceInput>,
        events: Sender<DapEvent>,
        wake: Sender<()>,
    ) -> Self {
        Self {
            input,
            input_tx,
            events,
            wake,
            client: None,
            config: None,
            breakpoints: BTreeMap::new(),
            function_breakpoints: Vec::new(),
            exception_filters: None,
            available_exception_filters: Vec::new(),
            supports_function_breakpoints: false,
            supports_restart_request: false,
            supports_terminate_request: false,
            reverse_requests: BTreeMap::new(),
            pending: std::collections::HashMap::new(),
            next_generation: 0,
            next_seq: 0,
            initialized: false,
            launch_sent: false,
            configuration_done: false,
            initialize_deadline: None,
            disconnect_deadline: None,
            stopping: false,
        }
    }

    fn request_id(&mut self) -> i64 {
        self.next_seq = self.next_seq.wrapping_add(1);
        self.next_seq
    }

    fn send_request(
        &mut self,
        command: &str,
        arguments: Option<Value>,
        pending: PendingRequest,
    ) -> bool {
        let seq = self.request_id();
        let Some(client) = self.client.as_ref() else {
            return false;
        };
        if !client
            .client
            .send(Message::request(seq, command, arguments))
        {
            return false;
        }
        self.pending.insert(seq, pending);
        true
    }

    fn send_response(
        &mut self,
        request_seq: i64,
        success: bool,
        command: String,
        message: Option<String>,
        body: Option<Value>,
    ) {
        let seq = self.request_id();
        if let Some(client) = self.client.as_ref() {
            client.client.send(Message::response(
                seq,
                request_seq,
                success,
                command,
                message,
                body,
            ));
        }
    }

    fn emit(&self, event: DapEvent) {
        if self.events.send(event).is_ok() {
            let _ = self.wake.send(());
        }
    }
}
