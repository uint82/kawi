use super::super::format::{variable_line, watch_line};
use super::super::types::{ConsoleEntry, WatchExpression};
use super::super::DebugReport;
use kawi_dap::{Scope, Thread, Variable};
use std::collections::{BTreeMap, VecDeque};

pub(crate) fn variables(
    scopes: &[Scope],
    variables: &BTreeMap<i64, Vec<Variable>>,
    totals: &BTreeMap<i64, usize>,
) -> DebugReport {
    let mut lines = vec!["Variables".to_string()];
    if scopes.is_empty() {
        lines.push("No scopes.".to_string());
    } else {
        for scope in scopes {
            if scope.variables_reference > 0 {
                lines.push(format!(
                    "{} [ref {}]",
                    scope.name, scope.variables_reference
                ));
            } else {
                lines.push(scope.name.clone());
            }
            let scope_variables = variables
                .get(&scope.variables_reference)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            push_variables(&mut lines, scope_variables, "  ");
            push_page_hint(
                &mut lines,
                scope.variables_reference,
                scope_variables.len(),
                totals,
            );
        }
    }
    DebugReport {
        topic: "debug-variables",
        lines,
    }
}

pub(crate) fn variable_children(reference: i64, variables: &[Variable]) -> DebugReport {
    let mut lines = vec![format!("Variables (reference {reference})")];
    push_variables(&mut lines, variables, "");
    DebugReport {
        topic: "debug-variables",
        lines,
    }
}

pub(crate) fn threads(threads: &[Thread], current: Option<i64>) -> DebugReport {
    let mut lines = vec!["Threads".to_string()];
    if threads.is_empty() {
        lines.push("No threads.".to_string());
    } else {
        for thread in threads {
            let marker = if current == Some(thread.id) { "*" } else { " " };
            lines.push(format!("[{marker}] {}: {}", thread.id, thread.name));
        }
    }
    DebugReport {
        topic: "debug-threads",
        lines,
    }
}

pub(crate) fn watches(watches: &[WatchExpression]) -> DebugReport {
    let mut lines = vec!["Watch expressions".to_string()];
    if watches.is_empty() {
        lines.push("No watch expressions.".to_string());
    } else {
        for (index, watch) in watches.iter().enumerate() {
            lines.push(watch_line(watch, index));
        }
    }
    DebugReport {
        topic: "debug-watches",
        lines,
    }
}

pub(crate) fn console(console: &VecDeque<ConsoleEntry>) -> DebugReport {
    let mut lines = vec!["Debug console".to_string()];
    if console.is_empty() {
        lines.push("No console entries.".to_string());
    } else {
        for entry in console {
            lines.push(format!("> {}", entry.input));
            let prefix = if entry.is_error { "error: " } else { "" };
            let kind = entry
                .kind
                .as_ref()
                .map(|kind| format!(": {kind}"))
                .unwrap_or_default();
            lines.push(format!(
                "  {prefix}{}{}",
                entry.output,
                reference_suffix(entry.variables_reference, &kind)
            ));
        }
    }
    DebugReport {
        topic: "debug-console",
        lines,
    }
}

fn push_variables(lines: &mut Vec<String>, variables: &[Variable], indent: &str) {
    if variables.is_empty() {
        lines.push(format!("{indent}No variables."));
        return;
    }
    for variable in variables {
        lines.push(format!("{indent}{}", variable_line(variable)));
    }
}

fn push_page_hint(
    lines: &mut Vec<String>,
    reference: i64,
    loaded: usize,
    totals: &BTreeMap<i64, usize>,
) {
    let Some(total) = totals.get(&reference).copied() else {
        return;
    };
    if total > loaded {
        lines.push(format!(
            "  {loaded}/{total} variables loaded; use :debug more {reference}"
        ));
    }
}

fn reference_suffix(reference: i64, kind: &str) -> String {
    if reference > 0 {
        format!("{kind} [ref {reference}]")
    } else {
        kind.to_string()
    }
}
