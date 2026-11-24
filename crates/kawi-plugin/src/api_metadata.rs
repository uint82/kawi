use serde::Serialize;

use crate::api_maturity::{
    is_process_available, ApiMaturity, ApiStability, Event, HostMethod, EVENT_ENTRIES,
    HOST_METHOD_ENTRIES,
};
use crate::capability::Capability;
use crate::protocol::{PROCESS_PROTOCOL_MINOR, PROTOCOL_MINOR, PROTOCOL_VERSION};

mod errors;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApiVersionMetadata {
    pub protocol: u64,
    pub protocol_minor: u64,
    pub process_protocol_minor: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ApiParameterMetadata {
    pub name: &'static str,
    pub ty: &'static str,
    pub required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ApiSignatureMetadata {
    pub parameters: &'static [ApiParameterMetadata],
    pub returns: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApiEntryMetadata {
    pub name: &'static str,
    pub since: u64,
    pub stability: &'static str,
    pub deprecated_since: Option<u64>,
    pub replacement: Option<&'static str>,
    pub capability: Option<&'static str>,
    pub process: bool,
    pub interactive: bool,
    pub mutates: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<ApiSignatureMetadata>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ApiErrorMetadata {
    pub name: &'static str,
    pub code: i64,
    pub description: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApiMetadata {
    pub version: ApiVersionMetadata,
    pub methods: Vec<ApiEntryMetadata>,
    pub events: Vec<ApiEntryMetadata>,
    pub error_types: Vec<ApiErrorMetadata>,
}

fn stability_label(maturity: ApiMaturity) -> &'static str {
    match maturity.stability {
        ApiStability::Stable => "stable",
        ApiStability::Experimental => "experimental",
        ApiStability::Deprecated => "deprecated",
    }
}

fn base_entry(
    name: &'static str,
    maturity: ApiMaturity,
    capability: Option<&'static str>,
) -> ApiEntryMetadata {
    ApiEntryMetadata {
        name,
        since: maturity.since,
        stability: stability_label(maturity),
        deprecated_since: maturity.deprecated_since,
        replacement: maturity.replacement,
        capability,
        process: is_process_available(maturity),
        interactive: false,
        mutates: false,
        signature: None,
    }
}

fn method_entry_metadata(entry: &HostMethod) -> ApiEntryMetadata {
    ApiEntryMetadata {
        interactive: entry.interactive,
        mutates: entry.mutates,
        signature: Some(signatures::method(entry.name)),
        ..base_entry(
            entry.name,
            entry.maturity,
            entry.capability.map(Capability::name),
        )
    }
}

fn event_entry_metadata(entry: &Event) -> ApiEntryMetadata {
    base_entry(entry.name, entry.maturity, Some(entry.capability.name()))
}

fn method_entries() -> Vec<ApiEntryMetadata> {
    HOST_METHOD_ENTRIES
        .iter()
        .map(method_entry_metadata)
        .collect()
}

fn event_entries() -> Vec<ApiEntryMetadata> {
    EVENT_ENTRIES.iter().map(event_entry_metadata).collect()
}

pub fn api_metadata() -> ApiMetadata {
    ApiMetadata {
        version: ApiVersionMetadata {
            protocol: PROTOCOL_VERSION,
            protocol_minor: PROTOCOL_MINOR,
            process_protocol_minor: PROCESS_PROTOCOL_MINOR,
        },
        methods: method_entries(),
        events: event_entries(),
        error_types: errors::ERROR_TYPES.to_vec(),
    }
}

#[cfg(test)]
mod tests;

mod signatures;
