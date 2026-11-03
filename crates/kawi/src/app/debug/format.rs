use super::types::{BreakpointSpec, WatchExpression};
use kawi_dap::{StackFrame, Variable};
use std::path::Path;

pub(super) fn variable_line(variable: &Variable) -> String {
    let kind = variable
        .kind
        .as_ref()
        .map(|kind| format!(": {kind}"))
        .unwrap_or_default();
    format!(
        "{}{} = {}{}",
        variable.name,
        kind,
        variable.value,
        reference_suffix(variable.variables_reference, "")
    )
}

pub(super) fn watch_line(watch: &WatchExpression, index: usize) -> String {
    format!("{}. {}", index.saturating_add(1), watch_value(watch))
}

pub(super) fn watch_value(watch: &WatchExpression) -> String {
    let result = watch.result.as_deref().unwrap_or("<not evaluated>");
    let kind = watch
        .kind
        .as_ref()
        .map(|kind| format!(": {kind}"))
        .unwrap_or_default();
    format!(
        "{} = {}{}{}",
        watch.expression,
        result,
        kind,
        reference_suffix(watch.variables_reference, "")
    )
}

pub(super) fn frame_line(frame: &StackFrame) -> String {
    let source = frame
        .source
        .as_deref()
        .map(|path| format!(" ({}:{}:{})", path.display(), frame.line, frame.column))
        .unwrap_or_default();
    format!("#{} {}{}", frame.id, frame.name, source)
}

pub(super) fn breakpoint_line(path: &Path, line: u32, spec: &BreakpointSpec) -> String {
    let mut label = format!("{}:{}", path.display(), line);
    if let Some(condition) = spec.condition.as_ref() {
        label.push_str(&format!(" if {condition}"));
    }
    if let Some(hit_condition) = spec.hit_condition.as_ref() {
        label.push_str(&format!(" hit {hit_condition}"));
    }
    if let Some(log_message) = spec.log_message.as_ref() {
        label.push_str(&format!(" log {log_message}"));
    }
    label
}

pub(super) fn reference_suffix(reference: i64, kind: &str) -> String {
    if reference > 0 {
        format!("{kind} [ref {reference}]")
    } else {
        kind.to_string()
    }
}
