use super::*;

const VARIABLE_PAGE_SIZE: i64 = 100;

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
        self.pending_report = Some(ReportKind::Variables);
        if !self.request_scopes(frame.id) {
            self.pending_report = None;
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
        self.expanded_variables.insert(variables_reference);
        self.pending_report = Some(ReportKind::Expand(variables_reference));
        if !self.request_variable_page(variables_reference, 0) {
            self.pending_report = None;
            return false;
        }
        true
    }

    pub(super) fn threads(&mut self) -> bool {
        if !matches!(self.status, DebugStatus::Running | DebugStatus::Stopped) {
            self.message = Some("debug: not running".to_string());
            return false;
        }
        self.pending_report = Some(ReportKind::Threads);
        if !self.request_threads() {
            self.pending_report = None;
            return false;
        }
        true
    }

    pub(super) fn refresh_threads(&mut self) -> bool {
        if !matches!(self.status, DebugStatus::Running | DebugStatus::Stopped) {
            self.message = Some("debug: not running".to_string());
            return false;
        }
        self.pending_report = None;
        self.request_threads()
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
        self.jump_on_stack = true;
        if !self.request_stack(thread_id) {
            self.message = Some("debug: service unavailable".to_string());
            return false;
        }
        true
    }

    pub(super) fn select_frame(&mut self, frame_id: i64) -> bool {
        let Some(frame) = self
            .stack
            .iter()
            .find(|frame| frame.id == frame_id)
            .cloned()
        else {
            self.message = Some(format!("debug: unknown frame {frame_id}"));
            return false;
        };
        self.current_frame = Some(frame.clone());
        self.pending_report = None;
        self.jump = frame.source.map(|path| DebugJump {
            path,
            line: frame.line.saturating_sub(1) as usize,
            column: frame.column.saturating_sub(1) as usize,
        });
        if !self.request_scopes(frame.id) {
            self.message = Some("debug: service unavailable".to_string());
            return false;
        }
        true
    }

    pub(super) fn toggle_variable(&mut self, variables_reference: i64) -> bool {
        if variables_reference <= 0 {
            self.message = Some("debug: invalid variable reference".to_string());
            return false;
        }
        if self.expanded_variables.remove(&variables_reference) {
            return true;
        }
        self.expanded_variables.insert(variables_reference);
        if self.variables.contains_key(&variables_reference) {
            return true;
        }
        if !self.request_variable_page(variables_reference, 0) {
            self.message = Some("debug: service unavailable".to_string());
            return false;
        }
        true
    }

    pub(super) fn load_more_variables(&mut self, variables_reference: i64) -> bool {
        let loaded = self
            .variables
            .get(&variables_reference)
            .map(Vec::len)
            .unwrap_or(0);
        let total = self
            .variable_totals
            .get(&variables_reference)
            .copied()
            .unwrap_or(loaded);
        if loaded >= total {
            self.message = Some("debug: no more variables".to_string());
            return false;
        }
        if !self.request_variable_page(variables_reference, loaded as i64) {
            self.message = Some("debug: service unavailable".to_string());
            return false;
        }
        true
    }

    fn request_scopes(&mut self, frame_id: i64) -> bool {
        let Some(service) = self.service.as_ref() else {
            return false;
        };
        self.scopes.clear();
        self.variables.clear();
        self.variable_totals.clear();
        self.expanded_variables.clear();
        self.pending_variables.clear();
        service.scopes(frame_id)
    }

    fn request_stack(&mut self, thread_id: i64) -> bool {
        let Some(service) = self.service.as_ref() else {
            return false;
        };
        self.pending_report = None;
        service.stack_trace(thread_id)
    }

    fn request_threads(&self) -> bool {
        self.service
            .as_ref()
            .is_some_and(|service| service.threads())
    }

    pub(super) fn request_pending_variables(&self) {
        let Some(service) = self.service.as_ref() else {
            return;
        };
        for (reference, start) in &self.pending_variables {
            service.variables_page(*reference, *start, VARIABLE_PAGE_SIZE);
        }
    }

    fn request_variable_page(&mut self, variables_reference: i64, start: i64) -> bool {
        let Some(service) = self.service.as_ref() else {
            return false;
        };
        self.pending_variables.insert(variables_reference, start);
        if service.variables_page(variables_reference, start, VARIABLE_PAGE_SIZE) {
            return true;
        }
        self.pending_variables.remove(&variables_reference);
        false
    }
}

pub(super) fn total_variables(named: Option<i64>, indexed: Option<i64>) -> Option<usize> {
    match (named, indexed) {
        (None, None) => None,
        (named, indexed) => Some(
            named
                .unwrap_or(0)
                .saturating_add(indexed.unwrap_or(0))
                .max(0) as usize,
        ),
    }
}
