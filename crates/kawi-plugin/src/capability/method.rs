use crate::api_maturity::{event_entry, host_method_entry, is_process_available};

use super::{Capability, CapabilityRegistry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodCapability {
    Unknown,
    None,
    Required(Capability),
}

pub fn is_process_host_method(method: &str) -> bool {
    host_method_entry(method).is_some_and(|entry| is_process_available(entry.maturity))
}

pub fn is_process_event(event: &str) -> bool {
    event_entry(event).is_some_and(|entry| is_process_available(entry.maturity))
}

pub fn host_method_capability(method: &str) -> MethodCapability {
    match host_method_entry(method) {
        None => MethodCapability::Unknown,
        Some(entry) => match entry.capability {
            Some(capability) => MethodCapability::Required(capability),
            None => MethodCapability::None,
        },
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
    event_entry(event).map(|entry| entry.capability)
}
