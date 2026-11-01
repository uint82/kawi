use super::{Capability, CapabilityRegistry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodCapability {
    Unknown,
    None,
    Required(Capability),
}

pub fn host_method_capability(method: &str) -> MethodCapability {
    match method {
        "editor.get_state" => MethodCapability::Required(Capability::EditorRead),
        "editor.apply_edit" => MethodCapability::Required(Capability::EditorWrite),
        "editor.execute" | "editor.list_commands" => {
            MethodCapability::Required(Capability::EditorExecute)
        }
        "ui.message" => MethodCapability::Required(Capability::UiMessage),
        "ui.input" | "ui.select" | "ui.confirm" => MethodCapability::Required(Capability::UiPrompt),
        "ui.progress_start" | "ui.progress_update" | "ui.progress_finish" => {
            MethodCapability::Required(Capability::UiProgress)
        }
        "ui.set_statusline" => MethodCapability::Required(Capability::UiStatusline),
        "editor.set_diagnostics" => MethodCapability::Required(Capability::DiagnosticsPublish),
        "editor.set_decorations" => MethodCapability::Required(Capability::EditorDecorations),
        "config.get" | "config.list" => MethodCapability::None,
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
    event_capability(event).is_none_or(|capability| registry.has(plugin, capability))
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
        "diagnostics_changed" => Some(Capability::DiagnosticsRead),
        _ => None,
    }
}
