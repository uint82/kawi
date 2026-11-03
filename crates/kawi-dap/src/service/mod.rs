use crate::client::{Client, ClientEvent};
use crate::protocol::Message;
use crate::ExpandedConfiguration;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

mod manager;
mod types;

pub use types::*;

const INITIALIZE_TIMEOUT: Duration = Duration::from_secs(10);
const DISCONNECT_TIMEOUT: Duration = Duration::from_secs(1);
const STACK_FRAME_LIMIT: i64 = 20;
const VARIABLE_PAGE_SIZE: i64 = 100;

pub struct DapService {
    commands: Sender<ServiceInput>,
    events: Receiver<DapEvent>,
    worker: Option<JoinHandle<()>>,
}

impl DapService {
    pub fn new(wake: Sender<()>) -> Result<Self> {
        let (commands, input) = mpsc::channel();
        let (event_tx, events) = mpsc::channel();
        let manager_commands = commands.clone();
        let worker = thread::Builder::new()
            .name("kawi-dap".to_string())
            .spawn(move || manager::Manager::new(input, manager_commands, event_tx, wake).run())
            .context("spawn dap manager")?;
        Ok(Self {
            commands,
            events,
            worker: Some(worker),
        })
    }

    pub fn start(&self, config: ExpandedConfiguration, breakpoints: Vec<BreakpointSet>) -> bool {
        self.start_request(StartRequest {
            config,
            breakpoints,
            function_breakpoints: Vec::new(),
            exception_filters: None,
        })
    }

    pub fn start_request(&self, request: StartRequest) -> bool {
        self.send(Command::Start(Box::new(request)))
    }

    pub fn stop(&self) -> bool {
        self.send(Command::Stop)
    }

    pub fn set_breakpoints(&self, path: PathBuf, breakpoints: Vec<SourceBreakpoint>) -> bool {
        self.send(Command::SetBreakpoints { path, breakpoints })
    }

    pub fn continue_thread(&self, thread_id: i64) -> bool {
        self.send(Command::Continue { thread_id })
    }

    pub fn step(&self, thread_id: i64, kind: StepKind) -> bool {
        self.send(Command::Step { thread_id, kind })
    }

    pub fn stack_trace(&self, thread_id: i64) -> bool {
        self.send(Command::StackTrace { thread_id })
    }

    pub fn scopes(&self, frame_id: i64) -> bool {
        self.send(Command::Scopes { frame_id })
    }

    pub fn variables(&self, variables_reference: i64) -> bool {
        self.variables_page(variables_reference, 0, VARIABLE_PAGE_SIZE)
    }

    pub fn variables_page(&self, variables_reference: i64, start: i64, count: i64) -> bool {
        self.send(Command::Variables {
            variables_reference,
            start,
            count,
        })
    }

    pub fn set_variable(&self, variables_reference: i64, name: String, value: String) -> bool {
        self.send(Command::SetVariable {
            variables_reference,
            name,
            value,
        })
    }

    pub fn restart(&self) -> bool {
        self.send(Command::Restart)
    }

    pub fn terminate(&self) -> bool {
        self.send(Command::Terminate)
    }

    pub fn threads(&self) -> bool {
        self.send(Command::Threads)
    }

    pub fn evaluate(&self, expression: String, frame_id: Option<i64>, context: String) -> bool {
        self.send(Command::Evaluate {
            expression,
            frame_id,
            context,
        })
    }

    pub fn set_function_breakpoints(&self, breakpoints: Vec<FunctionBreakpoint>) -> bool {
        self.send(Command::SetFunctionBreakpoints { breakpoints })
    }

    pub fn set_exception_filters(&self, filters: Vec<String>) -> bool {
        self.send(Command::SetExceptionFilters { filters })
    }

    pub fn respond_run_in_terminal(
        &self,
        request_seq: i64,
        process_id: Option<u32>,
        message: Option<String>,
    ) -> bool {
        self.send(Command::RunInTerminalResponse {
            request_seq,
            process_id,
            message,
        })
    }

    pub fn try_recv(&self) -> Option<DapEvent> {
        self.events.try_recv().ok()
    }

    fn send(&self, command: Command) -> bool {
        self.commands.send(ServiceInput::Command(command)).is_ok()
    }
}

impl Drop for DapService {
    fn drop(&mut self) {
        let _ = self.commands.send(ServiceInput::Command(Command::Shutdown));
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub(crate) enum ServiceInput {
    Command(Command),
    Client(ClientEvent),
}

pub(crate) enum Command {
    Start(Box<StartRequest>),
    Stop,
    SetBreakpoints {
        path: PathBuf,
        breakpoints: Vec<SourceBreakpoint>,
    },
    Continue {
        thread_id: i64,
    },
    Step {
        thread_id: i64,
        kind: StepKind,
    },
    StackTrace {
        thread_id: i64,
    },
    Scopes {
        frame_id: i64,
    },
    Variables {
        variables_reference: i64,
        start: i64,
        count: i64,
    },
    SetVariable {
        variables_reference: i64,
        name: String,
        value: String,
    },
    Restart,
    Terminate,
    Threads,
    Evaluate {
        expression: String,
        frame_id: Option<i64>,
        context: String,
    },
    SetFunctionBreakpoints {
        breakpoints: Vec<FunctionBreakpoint>,
    },
    SetExceptionFilters {
        filters: Vec<String>,
    },
    RunInTerminalResponse {
        request_seq: i64,
        process_id: Option<u32>,
        message: Option<String>,
    },
    Shutdown,
}
