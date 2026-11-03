use crate::ExpandedConfiguration;
use std::collections::BTreeMap;
use std::path::PathBuf;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartRequest {
    pub config: ExpandedConfiguration,
    pub breakpoints: Vec<BreakpointSet>,
    pub function_breakpoints: Vec<FunctionBreakpoint>,
    pub exception_filters: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionBreakpoint {
    pub name: String,
    pub condition: Option<String>,
    pub hit_condition: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExceptionFilter {
    pub filter: String,
    pub label: String,
    pub default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thread {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunInTerminalRequest {
    pub request_seq: i64,
    pub kind: Option<String>,
    pub title: Option<String>,
    pub cwd: Option<PathBuf>,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub args_can_be_interpreted_by_shell: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepKind {
    Next,
    In,
    Out,
}

impl StepKind {
    pub(crate) fn command(self) -> &'static str {
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
    Threads {
        threads: Vec<Thread>,
    },
    Evaluated {
        expression: String,
        context: String,
        result: String,
        kind: Option<String>,
        variables_reference: i64,
    },
    EvaluationFailed {
        expression: String,
        context: String,
        message: String,
    },
    ExceptionFilters {
        filters: Vec<ExceptionFilter>,
    },
    RunInTerminal(RunInTerminalRequest),
    Error(String),
    Log(String),
}
