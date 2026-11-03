use super::variables::total_variables;
use super::*;

impl DebugUi {
    pub(super) fn apply_scopes(&mut self, frame_id: i64, scopes: Vec<Scope>) {
        let wants_report = self.pending_report == Some(ReportKind::Variables);
        if self.pending_report.is_some() && !wants_report {
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
        self.variable_totals.clear();
        self.expanded_variables.clear();
        self.pending_variables.clear();
        for scope in &self.scopes {
            if scope.variables_reference <= 0 {
                continue;
            }
            self.expanded_variables.insert(scope.variables_reference);
            if let Some(total) = total_variables(scope.named_variables, scope.indexed_variables) {
                self.variable_totals
                    .insert(scope.variables_reference, total);
            }
            self.pending_variables.insert(scope.variables_reference, 0);
        }
        if self.pending_variables.is_empty() {
            if wants_report {
                self.pending_report = None;
                self.set_report(report::variables(
                    &self.scopes,
                    &self.variables,
                    &self.variable_totals,
                ));
            }
            return;
        }
        self.request_pending_variables();
    }

    pub(super) fn apply_variables(
        &mut self,
        variables_reference: i64,
        variables: Vec<Variable>,
        start: i64,
    ) {
        self.store_variables(variables_reference, variables, start);
        self.pending_variables.remove(&variables_reference);
        self.finish_variable_report(variables_reference, start);
    }

    fn store_variables(&mut self, variables_reference: i64, variables: Vec<Variable>, start: i64) {
        for variable in &variables {
            if variable.variables_reference <= 0 {
                continue;
            }
            if let Some(total) =
                total_variables(variable.named_variables, variable.indexed_variables)
            {
                self.variable_totals
                    .insert(variable.variables_reference, total);
            }
        }
        if start == 0 {
            self.variables.insert(variables_reference, variables);
            return;
        }
        self.variables
            .entry(variables_reference)
            .or_default()
            .extend(variables);
    }

    fn finish_variable_report(&mut self, variables_reference: i64, start: i64) {
        match self.pending_report {
            Some(ReportKind::Variables) => {
                if self.pending_variables.is_empty() {
                    self.pending_report = None;
                    self.set_report(report::variables(
                        &self.scopes,
                        &self.variables,
                        &self.variable_totals,
                    ));
                }
            }
            Some(ReportKind::Expand(reference))
                if reference == variables_reference && start == 0 =>
            {
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

    pub(super) fn apply_variable_set(
        &mut self,
        variables_reference: i64,
        name: String,
        value: String,
        kind: Option<String>,
        new_reference: i64,
    ) {
        if let Some(variables) = self.variables.get_mut(&variables_reference) {
            if let Some(variable) = variables.iter_mut().find(|variable| variable.name == name) {
                variable.value = value;
                variable.kind = kind;
                variable.variables_reference = new_reference;
            }
        }
        self.message = Some(format!("debug: {name} updated"));
    }

    pub(super) fn apply_threads(&mut self, threads: Vec<Thread>) {
        self.session.threads = threads;
        if self.pending_report == Some(ReportKind::Threads) {
            self.pending_report = None;
            self.set_report(report::threads(self.thread_list(), self.thread_id));
        }
    }
}
