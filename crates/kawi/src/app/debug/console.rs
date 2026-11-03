use super::*;

impl DebugUi {
    pub(super) fn evaluate(&mut self, expression: String) -> bool {
        if !matches!(self.status, DebugStatus::Running | DebugStatus::Stopped) {
            self.message = Some("debug: not running".to_string());
            return false;
        }
        let Some(service) = self.service.as_ref() else {
            self.message = Some("debug: service unavailable".to_string());
            return false;
        };
        if !service.evaluate(expression.clone(), self.frame_id(), "repl".to_string()) {
            self.message = Some("debug: service unavailable".to_string());
            return false;
        }
        self.message = Some(format!("debug: evaluating {expression}"));
        true
    }

    pub(super) fn add_watch(&mut self, expression: String) -> bool {
        if self
            .watches
            .iter()
            .any(|watch| watch.expression == expression)
        {
            self.message = Some(format!("debug: watch already exists: {expression}"));
            return false;
        }
        self.watches.push(WatchExpression::new(expression.clone()));
        self.evaluate_watch(&expression);
        self.message = Some(format!("debug: watch added: {expression}"));
        true
    }

    pub(super) fn edit_watch(&mut self, index: usize, expression: String) -> bool {
        let position = index.saturating_sub(1);
        if position >= self.watches.len() {
            self.message = Some(format!("debug: watch index out of range: {index}"));
            return false;
        }
        if self
            .watches
            .iter()
            .enumerate()
            .any(|(current, watch)| current != position && watch.expression == expression)
        {
            self.message = Some(format!("debug: watch already exists: {expression}"));
            return false;
        }
        let watch = &mut self.watches[position];
        watch.expression = expression.clone();
        watch.clear();
        self.evaluate_watch(&expression);
        self.message = Some(format!("debug: watch {index} updated: {expression}"));
        true
    }

    pub(super) fn remove_watch(&mut self, expression: &str) -> bool {
        let before = self.watches.len();
        self.watches.retain(|watch| watch.expression != expression);
        if self.watches.len() == before {
            self.message = Some(format!("debug: watch not found: {expression}"));
            return false;
        }
        self.message = Some(format!("debug: watch removed: {expression}"));
        true
    }

    pub(super) fn show_watches(&mut self) {
        self.set_report(report::watches(&self.watches));
    }

    pub(super) fn show_console(&mut self) {
        self.set_report(report::console(&self.console));
    }

    pub(super) fn evaluate_watches(&self) {
        if self.status != DebugStatus::Stopped {
            return;
        }
        let Some(frame_id) = self.frame_id() else {
            return;
        };
        let Some(service) = self.service.as_ref() else {
            return;
        };
        for watch in &self.watches {
            service.evaluate(
                watch.expression.clone(),
                Some(frame_id),
                "watch".to_string(),
            );
        }
    }

    fn evaluate_watch(&self, expression: &str) {
        if self.status != DebugStatus::Stopped {
            return;
        }
        let Some(service) = self.service.as_ref() else {
            return;
        };
        service.evaluate(expression.to_string(), self.frame_id(), "watch".to_string());
    }

    pub(super) fn apply_evaluated(
        &mut self,
        expression: String,
        context: String,
        result: String,
        kind: Option<String>,
        variables_reference: i64,
    ) {
        let result = compact_value(&result);
        if context == "watch" {
            if self.status == DebugStatus::Stopped {
                self.apply_watch_result(&expression, result, kind, variables_reference);
            }
            return;
        }
        self.push_console(ConsoleEntry {
            input: expression.clone(),
            output: result.clone(),
            kind,
            variables_reference,
            is_error: false,
        });
        self.message = Some(format!("debug: {expression} = {result}"));
    }

    pub(super) fn apply_evaluation_failed(
        &mut self,
        expression: String,
        context: String,
        message: String,
    ) {
        let message = compact_value(&message);
        if context == "watch" {
            if self.status == DebugStatus::Stopped {
                self.apply_watch_failure(&expression, &message);
            }
            self.message = Some(format!("debug: {expression}: {message}"));
            return;
        }
        self.push_console(ConsoleEntry {
            input: expression.clone(),
            output: message.clone(),
            kind: None,
            variables_reference: 0,
            is_error: true,
        });
        self.message = Some(format!("debug: {expression}: {message}"));
    }

    fn apply_watch_result(
        &mut self,
        expression: &str,
        result: String,
        kind: Option<String>,
        variables_reference: i64,
    ) {
        for watch in self
            .watches
            .iter_mut()
            .filter(|watch| watch.expression == expression)
        {
            watch.result = Some(result.clone());
            watch.kind = kind.clone();
            watch.variables_reference = variables_reference;
        }
    }

    fn apply_watch_failure(&mut self, expression: &str, message: &str) {
        for watch in self
            .watches
            .iter_mut()
            .filter(|watch| watch.expression == expression)
        {
            watch.result = Some(format!("error: {message}"));
            watch.kind = None;
            watch.variables_reference = 0;
        }
    }

    fn frame_id(&self) -> Option<i64> {
        if self.status != DebugStatus::Stopped {
            return None;
        }
        self.current_frame.as_ref().map(|frame| frame.id)
    }

    fn push_console(&mut self, entry: ConsoleEntry) {
        if self.console.len() == MAX_CONSOLE_LINES {
            self.console.pop_front();
        }
        self.console.push_back(entry);
    }
}

fn compact_value(value: &str) -> String {
    value.lines().collect::<Vec<_>>().join(" ")
}
