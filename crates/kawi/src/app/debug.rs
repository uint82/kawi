use super::lsp::editor_path;
use crate::state::StatePaths;
use kawi_dap::{
    DapEvent, DapService, DebugConfig, DebugContext, DebugStatus, ExceptionFilter, Scope,
    StackFrame, StepKind, Thread, Variable,
};
use kawi_editor::Editor;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

mod app;
mod breakpoints;
mod console;
mod events;
mod report;
mod session;
#[cfg(test)]
mod tests;
mod types;
mod variables;

use types::{BreakpointSpec, ConsoleEntry, DebugSessionState, ReportKind, WatchExpression};

const MAX_OUTPUT_LINES: usize = 1_000;
const MAX_CONSOLE_LINES: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DebugReport {
    pub(super) topic: &'static str,
    pub(super) lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DebugJump {
    pub(super) path: PathBuf,
    pub(super) line: usize,
    pub(super) column: usize,
}

pub(super) struct DebugUi {
    service: Option<DapService>,
    config: DebugConfig,
    workspace: PathBuf,
    status: DebugStatus,
    message: Option<String>,
    report: Option<DebugReport>,
    jump: Option<DebugJump>,
    breakpoints: BTreeMap<PathBuf, BTreeMap<u32, BreakpointSpec>>,
    verified: BTreeSet<(PathBuf, u32)>,
    function_breakpoints: BTreeSet<String>,
    enabled_exception_filters: BTreeSet<String>,
    exception_filters_touched: bool,
    stack: Vec<StackFrame>,
    scopes: Vec<Scope>,
    variables: BTreeMap<i64, Vec<Variable>>,
    pending_variables: BTreeSet<i64>,
    current_frame: Option<StackFrame>,
    thread_id: Option<i64>,
    session: DebugSessionState,
    watches: Vec<WatchExpression>,
    console: VecDeque<ConsoleEntry>,
    pending_report: Option<ReportKind>,
    jump_on_stack: bool,
    output: VecDeque<String>,
    configuration: Option<String>,
}

impl Default for DebugUi {
    fn default() -> Self {
        Self {
            service: None,
            config: DebugConfig::default(),
            workspace: PathBuf::new(),
            status: DebugStatus::Idle,
            message: None,
            report: None,
            jump: None,
            breakpoints: BTreeMap::new(),
            verified: BTreeSet::new(),
            function_breakpoints: BTreeSet::new(),
            enabled_exception_filters: BTreeSet::new(),
            exception_filters_touched: false,
            stack: Vec::new(),
            scopes: Vec::new(),
            variables: BTreeMap::new(),
            pending_variables: BTreeSet::new(),
            current_frame: None,
            thread_id: None,
            session: DebugSessionState::new(),
            watches: Vec::new(),
            console: VecDeque::new(),
            pending_report: None,
            jump_on_stack: false,
            output: VecDeque::new(),
            configuration: None,
        }
    }
}

impl DebugUi {
    pub(super) fn load(
        &mut self,
        paths: &StatePaths,
        workspace: PathBuf,
        wake: Sender<()>,
    ) -> Vec<String> {
        let project = workspace.join(".kawi/debug.toml");
        self.config = DebugConfig::load(&[paths.debug.clone(), project]);
        self.workspace = workspace;
        self.service = match DapService::new(wake) {
            Ok(service) => Some(service),
            Err(error) => {
                self.message = Some(format!("debug: {error}"));
                None
            }
        };
        self.config
            .warnings()
            .iter()
            .map(|warning| warning.format())
            .collect()
    }

    pub(super) fn poll_events(&mut self) -> bool {
        let Some(service) = self.service.as_ref() else {
            return false;
        };
        let events: Vec<DapEvent> = std::iter::from_fn(|| service.try_recv()).collect();
        let mut changed = false;
        for event in events {
            changed = true;
            self.apply_event(event);
        }
        changed
    }

    pub(super) fn take_message(&mut self) -> Option<String> {
        self.message.take()
    }

    pub(super) fn take_report(&mut self) -> Option<DebugReport> {
        self.report.take()
    }

    pub(super) fn take_jump(&mut self) -> Option<DebugJump> {
        self.jump.take()
    }

    pub(super) fn display(&self) -> Option<&'static str> {
        match self.status {
            DebugStatus::Idle => None,
            DebugStatus::Starting => Some("dap: starting"),
            DebugStatus::Running => Some("dap: running"),
            DebugStatus::Stopped => Some("dap: stopped"),
            DebugStatus::Terminated => Some("dap: terminated"),
            DebugStatus::Failed => Some("dap: failed"),
        }
    }

    pub(super) fn sign_for(&self, path: &Path, row: usize) -> Option<DebugSign> {
        if let Some(frame) = self.current_frame.as_ref() {
            if same_path(frame.source.as_deref(), path)
                && frame.line.saturating_sub(1) as usize == row
            {
                return Some(DebugSign::Current);
            }
        }
        let line = row as u32;
        self.breakpoints
            .get(path)
            .is_some_and(|breakpoints| breakpoints.contains_key(&line))
            .then_some(if self.verified.contains(&(path.to_path_buf(), line)) {
                DebugSign::Breakpoint
            } else {
                DebugSign::Unverified
            })
    }

    pub(super) fn has_signs(&self, path: &Path) -> bool {
        self.current_frame
            .as_ref()
            .is_some_and(|frame| same_path(frame.source.as_deref(), path))
            || self
                .breakpoints
                .get(path)
                .is_some_and(|breakpoints| !breakpoints.is_empty())
    }

    fn clear_frame_state(&mut self) {
        self.stack.clear();
        self.scopes.clear();
        self.variables.clear();
        self.pending_variables.clear();
        self.current_frame = None;
        self.session.threads.clear();
    }

    pub(super) fn reset_session_state(&mut self) {
        self.clear_frame_state();
        self.thread_id = None;
        self.jump_on_stack = false;
        self.pending_report = None;
        self.output.clear();
        self.console.clear();
        self.verified.clear();
        self.session.exception_filters.clear();
        if !self.exception_filters_touched {
            self.enabled_exception_filters.clear();
        }
        for watch in &mut self.watches {
            watch.clear();
        }
    }

    fn set_report(&mut self, report: DebugReport) {
        self.report = Some(report);
    }

    fn exception_filters(&self) -> &[ExceptionFilter] {
        &self.session.exception_filters
    }

    fn thread_list(&self) -> &[Thread] {
        &self.session.threads
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DebugSign {
    Breakpoint,
    Unverified,
    Current,
}

fn same_path(left: Option<&Path>, right: &Path) -> bool {
    left.is_some_and(|left| {
        std::path::absolute(left).ok().as_deref() == std::path::absolute(right).ok().as_deref()
    })
}
