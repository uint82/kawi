use crate::api_maturity::{event_since, host_method_since};

use super::{Capability, CapabilityRegistry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodCapability {
    Unknown,
    None,
    Required(Capability),
}

pub fn is_process_host_method(method: &str) -> bool {
    host_method_since(method).is_some_and(|since| since <= crate::protocol::PROCESS_PROTOCOL_MINOR)
}

pub fn is_process_event(event: &str) -> bool {
    event_since(event).is_some_and(|since| since <= crate::protocol::PROCESS_PROTOCOL_MINOR)
}

pub fn host_method_capability(method: &str) -> MethodCapability {
    match method {
        "editor.get_state" => MethodCapability::Required(Capability::EditorRead),
        "editor.apply_edit" => MethodCapability::Required(Capability::EditorWrite),
        "buffer.list" | "buffer.get" => MethodCapability::Required(Capability::EditorBuffers),
        "buffer.open" | "buffer.apply_edit" | "buffer.create" | "buffer.close" => {
            MethodCapability::Required(Capability::EditorBuffersWrite)
        }
        "window.list" | "window.get" | "window.focus" | "window.open" | "window.close" => {
            MethodCapability::Required(Capability::EditorWindows)
        }
        "editor.set_cursor" | "editor.set_selection" | "editor.set_mode" => {
            MethodCapability::Required(Capability::EditorControl)
        }
        "options.list" | "options.get" => MethodCapability::Required(Capability::OptionsRead),
        "options.set" | "options.reset" => MethodCapability::Required(Capability::OptionsWrite),
        "timer.after" | "timer.every" | "timer.cancel" | "timer.list" => {
            MethodCapability::Required(Capability::EditorSchedule)
        }
        "variable.get" | "variable.set" | "variable.delete" | "variable.list" => {
            MethodCapability::Required(Capability::EditorVariables)
        }
        "editor.execute" | "editor.list_commands" => {
            MethodCapability::Required(Capability::EditorExecute)
        }
        "command.register" | "command.unregister" | "command.list" => {
            MethodCapability::Required(Capability::EditorCommands)
        }
        "keymap.set" | "keymap.remove" | "keymap.list" => {
            MethodCapability::Required(Capability::EditorKeymaps)
        }
        "autocmd.register" | "autocmd.remove" | "autocmd.list" | "autocmd.clear" => {
            MethodCapability::Required(Capability::EditorAutocmds)
        }
        "editor.emit_user_event" => MethodCapability::Required(Capability::EditorAutocmds),
        "ui.message" => MethodCapability::Required(Capability::UiMessage),
        "ui.input" | "ui.select" | "ui.confirm" => MethodCapability::Required(Capability::UiPrompt),
        "ui.progress_start" | "ui.progress_update" | "ui.progress_finish" => {
            MethodCapability::Required(Capability::UiProgress)
        }
        "ui.set_statusline" => MethodCapability::Required(Capability::UiStatusline),
        "ui.panel_open" | "ui.panel_update" | "ui.panel_focus" | "ui.panel_close" => {
            MethodCapability::Required(Capability::UiPanel)
        }
        "ui.overlay_open" | "ui.overlay_update" | "ui.overlay_wait" | "ui.overlay_close" => {
            MethodCapability::Required(Capability::UiOverlay)
        }
        "ui.picker" => MethodCapability::Required(Capability::UiPicker),
        "editor.set_diagnostics" => MethodCapability::Required(Capability::DiagnosticsPublish),
        "editor.set_decorations" => MethodCapability::Required(Capability::EditorDecorations),
        "config.get" | "config.list" => MethodCapability::None,
        "process.open"
        | "process.write"
        | "process.close_stdin"
        | "process.signal"
        | "process.kill"
        | "process.status"
        | "process.poll" => MethodCapability::Required(Capability::ProcessSpawn),
        "task.list" | "task.start" | "task.status" | "task.poll" | "task.cancel" => {
            MethodCapability::Required(Capability::TaskRun)
        }
        _ => MethodCapability::Unknown,
    }
}

pub fn is_host_method_allowed(registry: &CapabilityRegistry, plugin: &str, method: &str) -> bool {
    match host_method_capability(method) {
        MethodCapability::None => true,
        MethodCapability::Required(capability) => registry.has(plugin, capability),
        MethodCapability::Unknown => false,
    }
}

pub fn advertised_host_methods(registry: &CapabilityRegistry, plugin: &str) -> Vec<&'static str> {
    crate::protocol::HOST_METHODS
        .iter()
        .copied()
        .filter(|method| is_host_method_allowed(registry, plugin, method))
        .collect()
}

pub fn is_event_allowed(registry: &CapabilityRegistry, plugin: &str, event: &str) -> bool {
    missing_event_capability(registry, plugin, event).is_none()
}

pub fn missing_event_capability(
    registry: &CapabilityRegistry,
    plugin: &str,
    event: &str,
) -> Option<Capability> {
    event_capability(event).filter(|capability| !registry.has(plugin, *capability))
}

pub fn advertised_events(registry: &CapabilityRegistry, plugin: &str) -> Vec<&'static str> {
    crate::protocol::EVENT_NAMES
        .iter()
        .copied()
        .filter(|event| is_event_allowed(registry, plugin, event))
        .collect()
}

pub fn event_capability(event: &str) -> Option<Capability> {
    match event {
        "buffer_changed" | "buffer_opened" | "file_opened" | "buffer_closed" | "file_saved"
        | "mode_changed" => Some(Capability::EditorRead),
        "buffer_enter" | "buffer_leave" => Some(Capability::EditorBuffers),
        "window_enter" | "window_leave" | "focus_gained" | "focus_lost" | "window_created"
        | "window_closed" => Some(Capability::EditorWindows),
        "cursor_moved" | "selection_changed" => Some(Capability::EditorControl),
        "diagnostics_changed" => Some(Capability::DiagnosticsRead),
        "language_changed" => Some(Capability::EditorRead),
        "user" => Some(Capability::EditorAutocmds),
        _ => None,
    }
}
