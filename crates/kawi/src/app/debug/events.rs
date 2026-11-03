use super::*;

mod lifecycle;
mod output;
mod stack;

impl DebugUi {
    pub(super) fn apply_event(&mut self, event: DapEvent) {
        match event {
            DapEvent::Status(status) => self.apply_status(status),
            DapEvent::Stopped {
                reason,
                thread_id,
                all_threads_stopped,
                description,
                text,
            } => self.apply_stopped(reason, thread_id, all_threads_stopped, description, text),
            DapEvent::Continued { .. } => self.apply_continued(),
            DapEvent::Exited { exit_code } => self.apply_exited(exit_code),
            DapEvent::Terminated => self.apply_terminated(),
            DapEvent::Output { category, output } => self.push_output(&category, &output),
            DapEvent::Breakpoint { breakpoint, .. } => {
                self.apply_breakpoint(breakpoint.source, breakpoint.line, breakpoint.verified)
            }
            event => self.apply_data_event(event),
        }
    }

    fn apply_data_event(&mut self, event: DapEvent) {
        match event {
            DapEvent::StackTrace { thread_id, frames } => self.apply_stack(thread_id, frames),
            DapEvent::Scopes { frame_id, scopes } => self.apply_scopes(frame_id, scopes),
            DapEvent::Variables {
                variables_reference,
                variables,
            } => self.apply_variables(variables_reference, variables),
            DapEvent::Threads { threads } => self.apply_threads(threads),
            DapEvent::Evaluated {
                expression,
                context,
                result,
                kind,
                variables_reference,
            } => self.apply_evaluated(expression, context, result, kind, variables_reference),
            DapEvent::EvaluationFailed {
                expression,
                context,
                message,
            } => self.apply_evaluation_failed(expression, context, message),
            DapEvent::ExceptionFilters { filters } => self.apply_exception_filters(filters),
            DapEvent::Error(message) => self.apply_error(message),
            DapEvent::Log(line) => self.push_output("adapter", &line),
            _ => {}
        }
    }

    fn apply_status(&mut self, status: DebugStatus) {
        self.status = status;
        if status == DebugStatus::Failed {
            self.verified.clear();
        }
    }

    fn apply_exited(&mut self, exit_code: i64) {
        self.message = Some(format!("debug: exited with code {exit_code}"));
    }

    fn apply_error(&mut self, message: String) {
        self.message = Some(format!("debug: {message}"));
        match self.pending_report {
            Some(ReportKind::Variables) => {
                self.pending_report = None;
                self.pending_variables.clear();
                self.set_report(report::variables(&self.scopes, &self.variables));
            }
            Some(ReportKind::Stack) | Some(ReportKind::Threads) | Some(ReportKind::Expand(_)) => {
                self.pending_report = None;
            }
            _ => {}
        }
    }

    fn apply_stack(&mut self, thread_id: i64, frames: Vec<StackFrame>) {
        if self.status != DebugStatus::Stopped || self.thread_id != Some(thread_id) {
            return;
        }
        self.stack = frames;
        self.current_frame = self.stack.first().cloned();
        self.evaluate_watches();
        if self.pending_report == Some(ReportKind::Stack) {
            self.pending_report = None;
            self.set_report(report::stack(&self.stack));
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
}
