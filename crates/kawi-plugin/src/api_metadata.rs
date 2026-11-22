use serde::Serialize;

use crate::api_maturity::{event_maturity, host_method_maturity, ApiMaturity};
use crate::protocol::{
    EVENT_NAMES, HOST_METHODS, PROCESS_PROTOCOL_MINOR, PROTOCOL_MINOR, PROTOCOL_VERSION,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApiVersionMetadata {
    pub protocol: u64,
    pub protocol_minor: u64,
    pub process_protocol_minor: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApiEntryMetadata {
    pub name: &'static str,
    pub since: u64,
    pub stability: &'static str,
    pub deprecated_since: Option<u64>,
    pub replacement: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApiMetadata {
    pub version: ApiVersionMetadata,
    pub methods: Vec<ApiEntryMetadata>,
    pub events: Vec<ApiEntryMetadata>,
}

fn stability_label(maturity: ApiMaturity) -> &'static str {
    if maturity.is_stable() {
        "stable"
    } else if maturity.is_experimental() {
        "experimental"
    } else {
        "deprecated"
    }
}

fn entry_metadata(name: &'static str, maturity: ApiMaturity) -> ApiEntryMetadata {
    ApiEntryMetadata {
        name,
        since: maturity.since,
        stability: stability_label(maturity),
        deprecated_since: maturity.deprecated_since,
        replacement: maturity.replacement,
    }
}

fn method_entries() -> Vec<ApiEntryMetadata> {
    HOST_METHODS
        .iter()
        .copied()
        .map(|name| {
            let maturity = host_method_maturity(name).expect("registry covers every host method");
            entry_metadata(name, maturity)
        })
        .collect()
}

fn event_entries() -> Vec<ApiEntryMetadata> {
    EVENT_NAMES
        .iter()
        .copied()
        .map(|name| {
            let maturity = event_maturity(name).expect("registry covers every event");
            entry_metadata(name, maturity)
        })
        .collect()
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
    }
}

#[cfg(test)]
mod tests;
