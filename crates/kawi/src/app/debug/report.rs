use super::DebugReport;
use kawi_dap::{DebugConfig, DebugStatus, StackFrame, Variable};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::PathBuf;

pub(super) fn breakpoints(breakpoints: &BTreeMap<PathBuf, BTreeSet<u32>>) -> DebugReport {
    let mut lines = vec!["Debug breakpoints".to_string()];
    if breakpoints.is_empty() {
        lines.push("No breakpoints.".to_string());
    } else {
        for (path, rows) in breakpoints {
            for row in rows {
                lines.push(format!("{}:{}", path.display(), row.saturating_add(1)));
            }
        }
    }
    DebugReport {
        topic: "debug-breakpoints",
        lines,
    }
}

pub(super) fn stack(frames: &[StackFrame]) -> DebugReport {
    let mut lines = vec!["Call stack".to_string()];
    if frames.is_empty() {
        lines.push("No stack frames.".to_string());
    } else {
        for frame in frames {
            let source = frame
                .source
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            lines.push(format!(
                "#{} {} {}:{}:{}",
                frame.id, frame.name, source, frame.line, frame.column
            ));
        }
    }
    DebugReport {
        topic: "debug-stack",
        lines,
    }
}

pub(super) fn variables(scope: Option<&str>, variables: &[Variable]) -> DebugReport {
    let mut lines = vec![match scope {
        Some(scope) => format!("Variables ({scope})"),
        None => "Variables".to_string(),
    }];
    if variables.is_empty() {
        lines.push("No variables.".to_string());
    } else {
        for variable in variables {
            let kind = variable
                .kind
                .as_ref()
                .map(|kind| format!(": {kind}"))
                .unwrap_or_default();
            lines.push(format!("{}{} = {}", variable.name, kind, variable.value));
        }
    }
    DebugReport {
        topic: "debug-variables",
        lines,
    }
}

pub(super) fn status(
    status: DebugStatus,
    configuration: Option<&str>,
    thread_id: Option<i64>,
    frame: Option<&StackFrame>,
    breakpoint_count: usize,
) -> DebugReport {
    let mut lines = vec![
        "Debug status".to_string(),
        format!("state: {}", status_label(status)),
    ];
    if let Some(configuration) = configuration {
        lines.push(format!("configuration: {configuration}"));
    }
    if let Some(thread_id) = thread_id {
        lines.push(format!("thread: {thread_id}"));
    }
    if let Some(frame) = frame {
        let source = frame
            .source
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        lines.push(format!(
            "frame: {} at {}:{}:{}",
            frame.name, source, frame.line, frame.column
        ));
    }
    lines.push(format!("breakpoints: {breakpoint_count}"));
    DebugReport {
        topic: "debug-status",
        lines,
    }
}

pub(super) fn configurations(config: &DebugConfig) -> DebugReport {
    let mut lines = vec!["Debug configurations".to_string()];
    let mut found = false;
    for configuration in config.configurations() {
        found = true;
        let languages = if configuration.languages.is_empty() {
            "any".to_string()
        } else {
            configuration.languages.join(", ")
        };
        lines.push(format!(
            "{}: adapter {}, request {}, languages {}",
            configuration.name, configuration.adapter, configuration.request, languages
        ));
    }
    if !found {
        lines.push("No debug configurations.".to_string());
    }
    DebugReport {
        topic: "debug-configurations",
        lines,
    }
}

pub(super) fn output(output: &VecDeque<String>) -> DebugReport {
    let mut lines = vec!["Debug output".to_string()];
    if output.is_empty() {
        lines.push("No debug output.".to_string());
    } else {
        lines.extend(output.iter().cloned());
    }
    DebugReport {
        topic: "debug-output",
        lines,
    }
}

fn status_label(status: DebugStatus) -> &'static str {
    match status {
        DebugStatus::Idle => "idle",
        DebugStatus::Starting => "starting",
        DebugStatus::Running => "running",
        DebugStatus::Stopped => "stopped",
        DebugStatus::Terminated => "terminated",
        DebugStatus::Failed => "failed",
    }
}
