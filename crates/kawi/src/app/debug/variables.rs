use super::*;

impl DebugUi {
    pub(super) fn variables(&mut self) -> bool {
        if self.status != DebugStatus::Stopped {
            self.message = Some("debug: not stopped".to_string());
            return false;
        }
        let Some(frame) = self.current_frame.as_ref() else {
            self.message = Some("debug: no current frame".to_string());
            return false;
        };
        let Some(service) = self.service.as_ref() else {
            return false;
        };
        self.scopes.clear();
        self.variables.clear();
        self.pending_variables.clear();
        self.pending_report = Some(ReportKind::Variables);
        if !service.scopes(frame.id) {
            self.pending_report = None;
            self.message = Some("debug: service unavailable".to_string());
            return false;
        }
        true
    }

    pub(super) fn expand_variables(&mut self, variables_reference: i64) -> bool {
        if variables_reference <= 0 {
            self.message = Some("debug: invalid variable reference".to_string());
            return false;
        }
        if self.status != DebugStatus::Stopped {
            self.message = Some("debug: not stopped".to_string());
            return false;
        }
        let Some(service) = self.service.as_ref() else {
            return false;
        };
        self.pending_report = Some(ReportKind::Expand(variables_reference));
        if !service.variables(variables_reference) {
            self.pending_report = None;
            self.message = Some("debug: service unavailable".to_string());
            return false;
        }
        true
    }

    pub(super) fn threads(&mut self) -> bool {
        if !matches!(self.status, DebugStatus::Running | DebugStatus::Stopped) {
            self.message = Some("debug: not running".to_string());
            return false;
        }
        let Some(service) = self.service.as_ref() else {
            return false;
        };
        self.pending_report = Some(ReportKind::Threads);
        if !service.threads() {
            self.pending_report = None;
            self.message = Some("debug: service unavailable".to_string());
            return false;
        }
        true
    }

    pub(super) fn select_thread(&mut self, thread_id: i64) -> bool {
        if !self
            .thread_list()
            .iter()
            .any(|thread| thread.id == thread_id)
        {
            self.message = Some(format!("debug: unknown thread {thread_id}"));
            return false;
        }
        self.thread_id = Some(thread_id);
        self.message = Some(format!("debug: thread {thread_id} selected"));
        true
    }

    pub(super) fn apply_scopes(&mut self, frame_id: i64, scopes: Vec<Scope>) {
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
        self.variables.clear();
        self.pending_variables.clear();
        for scope in &self.scopes {
            if scope.variables_reference > 0 {
                self.pending_variables.insert(scope.variables_reference);
            }
        }
        if self.pending_variables.is_empty() {
            self.pending_report = None;
            self.set_report(report::variables(&self.scopes, &self.variables));
            return;
        }
        self.request_scope_variables();
    }

    fn request_scope_variables(&self) {
        let Some(service) = self.service.as_ref() else {
            return;
        };
        for reference in self.pending_variables.iter().copied() {
            service.variables(reference);
        }
    }

    pub(super) fn apply_variables(&mut self, variables_reference: i64, variables: Vec<Variable>) {
        self.variables.insert(variables_reference, variables);
        match self.pending_report {
            Some(ReportKind::Variables) => {
                self.pending_variables.remove(&variables_reference);
                if self.pending_variables.is_empty() {
                    self.pending_report = None;
                    self.set_report(report::variables(&self.scopes, &self.variables));
                }
            }
            Some(ReportKind::Expand(reference)) if reference == variables_reference => {
                self.pending_report = None;
                self.set_report(report::variable_children(
                    reference,
                    self.variables
                        .get(&variables_reference)
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                ));
            }
            _ => {}
        }
    }

    pub(super) fn apply_threads(&mut self, threads: Vec<Thread>) {
        self.session.threads = threads;
        if self.pending_report == Some(ReportKind::Threads) {
            self.pending_report = None;
            self.set_report(report::threads(self.thread_list(), self.thread_id));
        }
    }
}
