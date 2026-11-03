use super::*;
use kawi_dap::{BreakpointSet, FunctionBreakpoint, SourceBreakpoint};

enum BreakpointField {
    Condition,
    HitCondition,
    LogMessage,
}

impl DebugUi {
    pub(crate) fn toggle_breakpoint(&mut self, editor: &Editor) -> bool {
        let Some((path, line)) = current_line(editor) else {
            self.message = Some("debug: no file name".to_string());
            return false;
        };
        let breakpoints = self.breakpoints.entry(path.clone()).or_default();
        let enabled = if breakpoints.remove(&line).is_some() {
            false
        } else {
            breakpoints.insert(line, BreakpointSpec::default());
            true
        };
        if breakpoints.is_empty() {
            self.breakpoints.remove(&path);
        }
        if !enabled {
            self.verified.remove(&(path.clone(), line));
        }
        self.sync_breakpoints(&path);
        self.message = Some(format!(
            "debug: breakpoint {} at {}:{}",
            if enabled { "set" } else { "removed" },
            path.display(),
            line.saturating_add(1)
        ));
        true
    }

    pub(super) fn set_breakpoint_condition(&mut self, editor: &Editor, condition: String) -> bool {
        self.update_breakpoint(editor, Some(condition), BreakpointField::Condition)
    }

    pub(super) fn set_breakpoint_hit_condition(
        &mut self,
        editor: &Editor,
        condition: String,
    ) -> bool {
        self.update_breakpoint(editor, Some(condition), BreakpointField::HitCondition)
    }

    pub(super) fn set_breakpoint_log_message(&mut self, editor: &Editor, message: String) -> bool {
        self.update_breakpoint(editor, Some(message), BreakpointField::LogMessage)
    }

    pub(super) fn clear_breakpoint_options(&mut self, editor: &Editor) -> bool {
        let Some((path, line)) = current_line(editor) else {
            self.message = Some("debug: no file name".to_string());
            return false;
        };
        let Some(spec) = self
            .breakpoints
            .get_mut(&path)
            .and_then(|breakpoints| breakpoints.get_mut(&line))
        else {
            self.message = Some("debug: no breakpoint on this line".to_string());
            return false;
        };
        *spec = BreakpointSpec::default();
        self.sync_breakpoints(&path);
        self.message = Some(format!(
            "debug: breakpoint options cleared at {}:{}",
            path.display(),
            line.saturating_add(1)
        ));
        true
    }

    pub(super) fn show_breakpoints(&mut self) {
        self.set_report(report::breakpoints(&self.breakpoints));
    }

    pub(super) fn toggle_function_breakpoint(&mut self, name: String) -> bool {
        let enabled = if self.function_breakpoints.remove(&name) {
            false
        } else {
            self.function_breakpoints.insert(name.clone());
            true
        };
        self.sync_function_breakpoints();
        self.message = Some(format!(
            "debug: function breakpoint {} {}",
            if enabled { "set on" } else { "removed from" },
            name
        ));
        true
    }

    pub(super) fn show_function_breakpoints(&mut self) {
        self.set_report(report::function_breakpoints(&self.function_breakpoints));
    }

    pub(super) fn toggle_exception_filter(&mut self, filter: String) -> bool {
        if !self
            .exception_filters()
            .iter()
            .any(|candidate| candidate.filter == filter)
        {
            self.message = Some(format!("debug: unknown exception filter {filter}"));
            return false;
        }
        self.exception_filters_touched = true;
        let enabled = if self.enabled_exception_filters.remove(&filter) {
            false
        } else {
            self.enabled_exception_filters.insert(filter.clone());
            true
        };
        self.sync_exception_filters();
        self.message = Some(format!(
            "debug: exception filter {} {}",
            if enabled { "enabled" } else { "disabled" },
            filter
        ));
        true
    }

    pub(super) fn apply_exception_filters(&mut self, filters: Vec<ExceptionFilter>) {
        self.session.exception_filters = filters;
        if !self.exception_filters_touched {
            self.enabled_exception_filters = self
                .exception_filters()
                .iter()
                .filter(|filter| filter.default)
                .map(|filter| filter.filter.clone())
                .collect();
        } else {
            let available: BTreeSet<String> = self
                .exception_filters()
                .iter()
                .map(|filter| filter.filter.clone())
                .collect();
            self.enabled_exception_filters
                .retain(|filter| available.contains(filter));
        }
    }

    pub(super) fn show_exception_filters(&mut self) {
        self.set_report(report::exception_filters(
            self.exception_filters(),
            &self.enabled_exception_filters,
        ));
    }

    pub(super) fn breakpoint_sets(&self) -> Vec<BreakpointSet> {
        self.breakpoints
            .iter()
            .map(|(path, breakpoints)| BreakpointSet {
                path: path.clone(),
                breakpoints: breakpoints
                    .iter()
                    .map(|(line, spec)| source_breakpoint(*line, spec))
                    .collect(),
            })
            .collect()
    }

    pub(super) fn function_breakpoint_values(&self) -> Vec<FunctionBreakpoint> {
        self.function_breakpoints
            .iter()
            .map(|name| FunctionBreakpoint {
                name: name.clone(),
                condition: None,
                hit_condition: None,
            })
            .collect()
    }

    pub(super) fn exception_filters_for_start(&self) -> Option<Vec<String>> {
        self.exception_filters_touched
            .then(|| self.enabled_exception_filters.iter().cloned().collect())
    }

    fn update_breakpoint(
        &mut self,
        editor: &Editor,
        value: Option<String>,
        field: BreakpointField,
    ) -> bool {
        let Some((path, line)) = current_line(editor) else {
            self.message = Some("debug: no file name".to_string());
            return false;
        };
        let spec = self
            .breakpoints
            .entry(path.clone())
            .or_default()
            .entry(line)
            .or_default();
        match field {
            BreakpointField::Condition => spec.condition = value,
            BreakpointField::HitCondition => spec.hit_condition = value,
            BreakpointField::LogMessage => spec.log_message = value,
        }
        self.sync_breakpoints(&path);
        self.message = Some(format!(
            "debug: breakpoint updated at {}:{}",
            path.display(),
            line.saturating_add(1)
        ));
        true
    }

    fn sync_breakpoints(&self, path: &Path) {
        let Some(service) = self.service.as_ref() else {
            return;
        };
        let breakpoints = self
            .breakpoints
            .get(path)
            .map(|breakpoints| {
                breakpoints
                    .iter()
                    .map(|(line, spec)| source_breakpoint(*line, spec))
                    .collect()
            })
            .unwrap_or_default();
        service.set_breakpoints(path.to_path_buf(), breakpoints);
    }

    fn sync_function_breakpoints(&self) {
        if let Some(service) = self.service.as_ref() {
            service.set_function_breakpoints(self.function_breakpoint_values());
        }
    }

    fn sync_exception_filters(&self) {
        if let Some(service) = self.service.as_ref() {
            service.set_exception_filters(self.enabled_exception_filters.iter().cloned().collect());
        }
    }

    pub(super) fn breakpoint_count(&self) -> usize {
        self.breakpoints.values().map(BTreeMap::len).sum()
    }
}

fn current_line(editor: &Editor) -> Option<(PathBuf, u32)> {
    Some((editor_path(editor)?, editor.row() as u32))
}

fn source_breakpoint(line: u32, spec: &BreakpointSpec) -> SourceBreakpoint {
    SourceBreakpoint {
        line: line.saturating_add(1),
        condition: spec.condition.clone(),
        hit_condition: spec.hit_condition.clone(),
        log_message: spec.log_message.clone(),
    }
}
