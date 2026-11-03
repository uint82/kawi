use crate::client::{Client, ClientEvent};
use crate::protocol::Message;
use crate::ExpandedConfiguration;
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

mod manager;

const INITIALIZE_TIMEOUT: Duration = Duration::from_secs(10);
const DISCONNECT_TIMEOUT: Duration = Duration::from_secs(1);
const STACK_FRAME_LIMIT: i64 = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceBreakpoint {
    pub line: u32,
    pub condition: Option<String>,
    pub hit_condition: Option<String>,
    pub log_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakpointSet {
    pub path: PathBuf,
    pub breakpoints: Vec<SourceBreakpoint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Breakpoint {
    pub verified: bool,
    pub line: Option<u32>,
    pub source: Option<PathBuf>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackFrame {
    pub id: i64,
    pub name: String,
    pub source: Option<PathBuf>,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub name: String,
    pub variables_reference: i64,
    pub expensive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub kind: Option<String>,
    pub variables_reference: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepKind {
    Next,
    In,
    Out,
}

impl StepKind {
    fn command(self) -> &'static str {
        match self {
            Self::Next => "next",
            Self::In => "stepIn",
            Self::Out => "stepOut",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugStatus {
    Idle,
    Starting,
    Running,
    Stopped,
    Terminated,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DapEvent {
    Status(DebugStatus),
    Stopped {
        reason: String,
        thread_id: Option<i64>,
        all_threads_stopped: bool,
        description: Option<String>,
        text: Option<String>,
    },
    Continued {
        thread_id: i64,
        all_threads_continued: bool,
    },
    Exited {
        exit_code: i64,
    },
    Terminated,
    Output {
        category: String,
        output: String,
    },
    Breakpoint {
        reason: String,
        breakpoint: Breakpoint,
    },
    StackTrace {
        thread_id: i64,
        frames: Vec<StackFrame>,
    },
    Scopes {
        frame_id: i64,
        scopes: Vec<Scope>,
    },
    Variables {
        variables_reference: i64,
        variables: Vec<Variable>,
    },
    Error(String),
    Log(String),
}

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
        self.send(Command::Start {
            config,
            breakpoints,
        })
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
        self.send(Command::Variables {
            variables_reference,
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
    Start {
        config: ExpandedConfiguration,
        breakpoints: Vec<BreakpointSet>,
    },
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
    },
    Shutdown,
}
