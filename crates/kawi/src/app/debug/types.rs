use kawi_dap::{ExceptionFilter, Thread};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct BreakpointSpec {
    pub(super) condition: Option<String>,
    pub(super) hit_condition: Option<String>,
    pub(super) log_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WatchExpression {
    pub(super) expression: String,
    pub(super) result: Option<String>,
    pub(super) kind: Option<String>,
    pub(super) variables_reference: i64,
}

impl WatchExpression {
    pub(super) fn new(expression: String) -> Self {
        Self {
            expression,
            result: None,
            kind: None,
            variables_reference: 0,
        }
    }

    pub(super) fn clear(&mut self) {
        self.result = None;
        self.kind = None;
        self.variables_reference = 0;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConsoleEntry {
    pub(super) input: String,
    pub(super) output: String,
    pub(super) kind: Option<String>,
    pub(super) variables_reference: i64,
    pub(super) is_error: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DebugSessionState {
    pub(super) exception_filters: Vec<ExceptionFilter>,
    pub(super) threads: Vec<Thread>,
}

impl DebugSessionState {
    pub(super) fn new() -> Self {
        Self {
            exception_filters: Vec::new(),
            threads: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ReportKind {
    Stack,
    Variables,
    Threads,
    Expand(i64),
}
