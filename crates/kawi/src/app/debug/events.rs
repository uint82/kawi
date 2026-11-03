use super::*;

impl DebugUi {
    pub(super) fn apply_event(&mut self, event: DapEvent) {
        match event {
            DapEvent::Status(status) => {
                self.status = status;
                if status == DebugStatus::Failed {
                    self.verified.clear();
                }
            }
            DapEvent::Stopped {
                reason,
                thread_id,
                all_threads_stopped,
                description,
                text,
            } => self.apply_stopped(reason, thread_id, all_threads_stopped, description, text),
            DapEvent::Continued { .. } => {
                self.status = DebugStatus::Running;
                self.clear_frame_state();
            }
            DapEvent::Exited { exit_code } => {
                self.message = Some(format!("debug: exited with code {exit_code}"));
            }
            DapEvent::Terminated => {
                self.status = DebugStatus::Terminated;
                self.thread_id = None;
                self.clear_frame_state();
                self.verified.clear();
                self.message = Some("debug: terminated".to_string());
            }
            DapEvent::Output { category, output } => self.push_output(&category, &output),
            DapEvent::Breakpoint { breakpoint, .. } => {
                self.apply_breakpoint(breakpoint.source, breakpoint.line, breakpoint.verified)
            }
            DapEvent::StackTrace { thread_id, frames } => self.apply_stack(thread_id, frames),
            DapEvent::Scopes { frame_id, scopes } => self.apply_scopes(frame_id, scopes),
            DapEvent::Variables {
                variables_reference,
                variables,
            } => self.apply_variables(variables_reference, variables),
            DapEvent::Error(message) => {
                self.message = Some(format!("debug: {message}"));
            }
            DapEvent::Log(line) => self.push_output("adapter", &line),
        }
    }

    fn apply_stopped(
        &mut self,
        reason: String,
        thread_id: Option<i64>,
        all_threads_stopped: bool,
        description: Option<String>,
        text: Option<String>,
    ) {
        self.status = DebugStatus::Stopped;
        self.thread_id = thread_id;
        self.clear_frame_state();
        self.jump_on_stack = true;
        let detail = text.or(description).unwrap_or_else(|| {
            if all_threads_stopped {
                reason.clone()
            } else {
                format!("{reason} (one thread)")
            }
        });
        self.message = Some(format!("debug: paused: {detail}"));
        if let (Some(thread_id), Some(service)) = (thread_id, self.service.as_ref()) {
            service.stack_trace(thread_id);
        }
    }

    fn apply_stack(&mut self, thread_id: i64, frames: Vec<StackFrame>) {
        if self.thread_id != Some(thread_id) {
            return;
        }
        self.stack = frames;
        self.current_frame = self.stack.first().cloned();
        if self.pending_report == Some(ReportKind::Stack) {
            self.pending_report = None;
            self.report = Some(report::stack(&self.stack));
            self.jump_on_stack = false;
            return;
        }
        if self.pending_report.is_some() {
            self.jump_on_stack = false;
            return;
        }
        if self.jump_on_stack {
            self.jump_on_stack = false;
            if let Some(frame) = self.current_frame.as_ref() {
                self.jump = frame.source.clone().map(|path| DebugJump {
                    path,
                    line: frame.line.saturating_sub(1) as usize,
                    column: frame.column.saturating_sub(1) as usize,
                });
            }
        }
    }

    fn apply_scopes(&mut self, frame_id: i64, scopes: Vec<Scope>) {
        if self.pending_report != Some(ReportKind::Variables) {
            return;
        }
        if self
            .current_frame
            .as_ref()
            .is_none_or(|frame| frame.id != frame_id)
        {
            return;
        }
        self.scopes = scopes;
        if let Some(scope) = self.scopes.first() {
            if let Some(service) = self.service.as_ref() {
                service.variables(scope.variables_reference);
                return;
            }
        }
        self.pending_report = None;
        self.report = Some(report::variables(None, &self.variables));
    }

    fn apply_variables(&mut self, variables_reference: i64, variables: Vec<Variable>) {
        if self.pending_report != Some(ReportKind::Variables) {
            return;
        }
        let scope = self
            .scopes
            .iter()
            .find(|scope| scope.variables_reference == variables_reference);
        if scope.is_none() {
            return;
        }
        self.variables = variables;
        self.pending_report = None;
        self.report = Some(report::variables(
            scope.map(|scope| scope.name.as_str()),
            &self.variables,
        ));
    }

    fn apply_breakpoint(&mut self, source: Option<PathBuf>, line: Option<u32>, verified: bool) {
        let (Some(source), Some(line)) = (source, line) else {
            return;
        };
        let source = std::path::absolute(&source).unwrap_or(source);
        let line = line.saturating_sub(1);
        if verified {
            self.verified.insert((source, line));
        } else {
            self.verified.remove(&(source, line));
        }
    }

    fn push_output(&mut self, category: &str, output: &str) {
        for line in output.lines() {
            if self.output.len() == MAX_OUTPUT_LINES {
                self.output.pop_front();
            }
            self.output.push_back(format!("[{category}] {line}"));
        }
    }
}
