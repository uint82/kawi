use super::*;

pub(super) fn push_section(entries: &mut Vec<PanelEntry>, label: &str) {
    entries.push(PanelEntry {
        depth: 0,
        label: label.to_string(),
        kind: DebugRowKind::Section,
        target: PanelTarget::None,
    });
}

pub(super) fn text_entry(depth: usize, label: String, error: bool) -> PanelEntry {
    PanelEntry {
        depth,
        label,
        kind: DebugRowKind::Console { error },
        target: PanelTarget::None,
    }
}

pub(super) fn push_threads(entries: &mut Vec<PanelEntry>, debug: &DebugUi) {
    push_section(entries, "Threads");
    if debug.session.threads.is_empty() {
        entries.push(text_entry(1, "No threads".to_string(), false));
        return;
    }
    for thread in &debug.session.threads {
        entries.push(PanelEntry {
            depth: 1,
            label: format!("{}: {}", thread.id, thread.name),
            kind: DebugRowKind::Thread {
                current: debug.thread_id == Some(thread.id),
            },
            target: PanelTarget::Thread(thread.id),
        });
    }
}

pub(super) fn push_stack(entries: &mut Vec<PanelEntry>, debug: &DebugUi) {
    push_section(entries, "Call Stack");
    if debug.stack.is_empty() {
        entries.push(text_entry(1, "No frames".to_string(), false));
        return;
    }
    for frame in &debug.stack {
        entries.push(PanelEntry {
            depth: 1,
            label: format::frame_line(frame),
            kind: DebugRowKind::Frame {
                current: debug
                    .current_frame
                    .as_ref()
                    .is_some_and(|current| current.id == frame.id),
            },
            target: PanelTarget::Frame(frame.id),
        });
    }
}

pub(super) fn push_variables(entries: &mut Vec<PanelEntry>, debug: &DebugUi) {
    push_section(entries, "Variables");
    if debug.scopes.is_empty() {
        entries.push(text_entry(1, "No scopes".to_string(), false));
        return;
    }
    for scope in &debug.scopes {
        let reference = scope.variables_reference;
        let expanded = debug.expanded_variables.contains(&reference);
        entries.push(PanelEntry {
            depth: 1,
            label: scope.name.clone(),
            kind: DebugRowKind::Variable {
                expandable: reference > 0,
                expanded,
            },
            target: if reference > 0 {
                PanelTarget::Variable(reference)
            } else {
                PanelTarget::None
            },
        });
        if reference > 0 && expanded {
            push_variable_children(entries, debug, reference, 2, &mut BTreeSet::new());
        }
    }
}

pub(super) fn push_variable_children(
    entries: &mut Vec<PanelEntry>,
    debug: &DebugUi,
    reference: i64,
    depth: usize,
    visited: &mut BTreeSet<i64>,
) {
    if !visited.insert(reference) {
        return;
    }
    let variables = debug
        .variables
        .get(&reference)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    for variable in variables {
        let child = variable.variables_reference;
        let expanded = child > 0 && debug.expanded_variables.contains(&child);
        entries.push(PanelEntry {
            depth,
            label: format::variable_line(variable),
            kind: DebugRowKind::Variable {
                expandable: child > 0,
                expanded,
            },
            target: if child > 0 {
                PanelTarget::Variable(child)
            } else {
                PanelTarget::None
            },
        });
        if expanded {
            push_variable_children(entries, debug, child, depth.saturating_add(1), visited);
        }
    }
    push_load_more(entries, debug, reference, depth);
    visited.remove(&reference);
}

pub(super) fn push_load_more(
    entries: &mut Vec<PanelEntry>,
    debug: &DebugUi,
    reference: i64,
    depth: usize,
) {
    let loaded = debug.variables.get(&reference).map(Vec::len).unwrap_or(0);
    let Some(total) = debug.variable_totals.get(&reference).copied() else {
        return;
    };
    if total <= loaded {
        return;
    }
    entries.push(PanelEntry {
        depth,
        label: format!("Load more ({loaded}/{total})"),
        kind: DebugRowKind::Action,
        target: PanelTarget::LoadMore(reference),
    });
}

pub(super) fn push_watches(entries: &mut Vec<PanelEntry>, debug: &DebugUi) {
    push_section(entries, "Watches");
    if debug.watches.is_empty() {
        entries.push(text_entry(1, "No watch expressions".to_string(), false));
        return;
    }
    for (index, watch) in debug.watches.iter().enumerate() {
        entries.push(PanelEntry {
            depth: 1,
            label: format::watch_line(watch, index),
            kind: DebugRowKind::Watch,
            target: PanelTarget::Watch(index),
        });
    }
}

pub(super) fn push_breakpoints(entries: &mut Vec<PanelEntry>, debug: &DebugUi) {
    push_section(entries, "Breakpoints");
    if debug.breakpoints.is_empty() {
        entries.push(text_entry(1, "No breakpoints".to_string(), false));
        return;
    }
    for (path, breakpoints) in &debug.breakpoints {
        for (line, spec) in breakpoints {
            entries.push(PanelEntry {
                depth: 1,
                label: format::breakpoint_line(path, *line, spec),
                kind: DebugRowKind::Breakpoint,
                target: PanelTarget::Breakpoint(path.clone(), *line),
            });
        }
    }
}

pub(super) fn push_function_breakpoints(entries: &mut Vec<PanelEntry>, debug: &DebugUi) {
    push_section(entries, "Function Breakpoints");
    if debug.function_breakpoints.is_empty() {
        entries.push(text_entry(1, "No function breakpoints".to_string(), false));
        return;
    }
    for name in &debug.function_breakpoints {
        entries.push(PanelEntry {
            depth: 1,
            label: name.clone(),
            kind: DebugRowKind::FunctionBreakpoint,
            target: PanelTarget::FunctionBreakpoint(name.clone()),
        });
    }
}

pub(super) fn push_exception_filters(entries: &mut Vec<PanelEntry>, debug: &DebugUi) {
    push_section(entries, "Exception Filters");
    if debug.exception_filters().is_empty() {
        entries.push(text_entry(1, "No exception filters".to_string(), false));
        return;
    }
    for filter in debug.exception_filters() {
        entries.push(PanelEntry {
            depth: 1,
            label: format!("{} ({})", filter.label, filter.filter),
            kind: DebugRowKind::ExceptionFilter {
                enabled: debug.enabled_exception_filters.contains(&filter.filter),
            },
            target: PanelTarget::ExceptionFilter(filter.filter.clone()),
        });
    }
}

pub(super) fn push_console(entries: &mut Vec<PanelEntry>, debug: &DebugUi) {
    push_section(entries, "Console");
    if debug.console.is_empty() {
        entries.push(text_entry(1, "No console entries".to_string(), false));
        return;
    }
    let start = debug.console.len().saturating_sub(PANEL_CONSOLE_LINES);
    for entry in debug.console.iter().skip(start) {
        entries.push(text_entry(1, format!("> {}", entry.input), false));
        entries.push(text_entry(2, entry.output.clone(), entry.is_error));
    }
}
