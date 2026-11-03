use super::*;
use serde_json::{json, Value};
use std::path::Path;

impl Manager {
    pub(super) fn send_launch(&mut self) {
        if self.launch_sent || self.stopping {
            return;
        }
        let Some(config) = self.config.as_ref() else {
            return;
        };
        let command = config.request.clone();
        let arguments = config.arguments.clone();
        if !self.send_request(&command, Some(arguments), PendingRequest::Launch) {
            self.finish_session(
                DebugStatus::Failed,
                Some("debug adapter is not accepting requests".to_string()),
            );
            return;
        }
        self.launch_sent = true;
    }

    pub(super) fn send_all_breakpoints(&mut self) {
        let breakpoints: Vec<(PathBuf, Vec<SourceBreakpoint>)> = self
            .breakpoints
            .iter()
            .map(|(path, breakpoints)| (path.clone(), breakpoints.clone()))
            .collect();
        for (path, breakpoints) in breakpoints {
            self.send_breakpoints(&path, &breakpoints);
        }
    }

    pub(super) fn send_breakpoints(&mut self, path: &Path, breakpoints: &[SourceBreakpoint]) {
        let payload: Vec<Value> = breakpoints.iter().map(breakpoint_payload).collect();
        let arguments = json!({
            "source": {"path": path.display().to_string()},
            "breakpoints": payload,
            "sourceModified": false,
        });
        self.send_request(
            "setBreakpoints",
            Some(arguments),
            PendingRequest::SetBreakpoints {
                path: path.to_path_buf(),
            },
        );
    }

    pub(super) fn send_function_breakpoints(&mut self) {
        if !self.supports_function_breakpoints {
            if !self.function_breakpoints.is_empty() {
                self.emit(DapEvent::Error(
                    "debug adapter does not support function breakpoints".to_string(),
                ));
            }
            return;
        }
        let payload: Vec<Value> = self
            .function_breakpoints
            .iter()
            .map(function_breakpoint_payload)
            .collect();
        self.send_request(
            "setFunctionBreakpoints",
            Some(json!({"breakpoints": payload})),
            PendingRequest::SetFunctionBreakpoints,
        );
    }

    pub(super) fn send_exception_breakpoints(&mut self) {
        if self.available_exception_filters.is_empty()
            && self.exception_filters.as_ref().is_none_or(Vec::is_empty)
        {
            return;
        }
        let filters = self.exception_filters.clone().unwrap_or_default();
        self.send_request(
            "setExceptionBreakpoints",
            Some(json!({"filters": filters})),
            PendingRequest::SetExceptionBreakpoints,
        );
    }

    pub(super) fn send_configuration_done(&mut self) {
        if self.send_request("configurationDone", None, PendingRequest::ConfigurationDone) {
            self.configuration_done = true;
        }
    }

    pub(super) fn maybe_emit_running(&self) {
        if self.launch_sent && self.configuration_done {
            self.emit(DapEvent::Status(DebugStatus::Running));
        }
    }
}

fn function_breakpoint_payload(breakpoint: &FunctionBreakpoint) -> Value {
    let mut value = json!({"name": breakpoint.name});
    if let Some(condition) = &breakpoint.condition {
        value["condition"] = Value::String(condition.clone());
    }
    if let Some(hit_condition) = &breakpoint.hit_condition {
        value["hitCondition"] = Value::String(hit_condition.clone());
    }
    value
}

fn breakpoint_payload(breakpoint: &SourceBreakpoint) -> Value {
    let mut value = json!({"line": breakpoint.line});
    if let Some(condition) = &breakpoint.condition {
        value["condition"] = Value::String(condition.clone());
    }
    if let Some(hit_condition) = &breakpoint.hit_condition {
        value["hitCondition"] = Value::String(hit_condition.clone());
    }
    if let Some(log_message) = &breakpoint.log_message {
        value["logMessage"] = Value::String(log_message.clone());
    }
    value
}
