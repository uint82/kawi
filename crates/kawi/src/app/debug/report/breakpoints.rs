use super::super::types::BreakpointSpec;
use super::super::DebugReport;
use kawi_dap::ExceptionFilter;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub(crate) fn breakpoints(
    breakpoints: &BTreeMap<PathBuf, BTreeMap<u32, BreakpointSpec>>,
) -> DebugReport {
    let mut lines = vec!["Debug breakpoints".to_string()];
    if breakpoints.is_empty() {
        lines.push("No breakpoints.".to_string());
    } else {
        for (path, breakpoints) in breakpoints {
            for (line, spec) in breakpoints {
                lines.push(format_breakpoint(path, *line, spec));
            }
        }
    }
    DebugReport {
        topic: "debug-breakpoints",
        lines,
    }
}

pub(crate) fn function_breakpoints(breakpoints: &BTreeSet<String>) -> DebugReport {
    let mut lines = vec!["Function breakpoints".to_string()];
    if breakpoints.is_empty() {
        lines.push("No function breakpoints.".to_string());
    } else {
        lines.extend(breakpoints.iter().cloned());
    }
    DebugReport {
        topic: "debug-function-breakpoints",
        lines,
    }
}

pub(crate) fn exception_filters(
    filters: &[ExceptionFilter],
    enabled: &BTreeSet<String>,
) -> DebugReport {
    let mut lines = vec!["Exception filters".to_string()];
    if filters.is_empty() {
        lines.push("No exception filters.".to_string());
    } else {
        for filter in filters {
            let marker = if enabled.contains(&filter.filter) {
                "*"
            } else {
                " "
            };
            let default = if filter.default { " (default)" } else { "" };
            lines.push(format!(
                "[{marker}] {}: {}{}",
                filter.filter, filter.label, default
            ));
        }
    }
    DebugReport {
        topic: "debug-exception-filters",
        lines,
    }
}

fn format_breakpoint(path: &Path, line: u32, spec: &BreakpointSpec) -> String {
    let mut line = format!("{}:{}", path.display(), line.saturating_add(1));
    if let Some(condition) = &spec.condition {
        line.push_str(&format!(" condition: {condition}"));
    }
    if let Some(hit_condition) = &spec.hit_condition {
        line.push_str(&format!(" hit: {hit_condition}"));
    }
    if let Some(log_message) = &spec.log_message {
        line.push_str(&format!(" log: {log_message}"));
    }
    line
}
