use serde::Serialize;

use crate::api_maturity::{event_maturity, host_method_maturity, ApiMaturity, ApiStability};
use crate::protocol::{
    EVENT_NAMES, HOST_METHODS, PROCESS_PROTOCOL_MINOR, PROTOCOL_MINOR, PROTOCOL_VERSION,
};

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

fn entry_metadata(name: &'static str, maturity: ApiMaturity) -> ApiEntryMetadata {
    ApiEntryMetadata {
        name,
        since: maturity.since,
        stability: stability_label(maturity),
        deprecated_since: maturity.deprecated_since,
        replacement: maturity.replacement,
        signature: None,
    }
}

fn method_metadata(name: &'static str, maturity: ApiMaturity) -> ApiEntryMetadata {
    ApiEntryMetadata {
        signature: Some(signatures::method(name)),
        ..entry_metadata(name, maturity)
    }
}

fn method_entries() -> Vec<ApiEntryMetadata> {
    HOST_METHODS
        .iter()
        .copied()
        .map(|name| {
            let maturity = host_method_maturity(name).expect("registry covers every host method");
            method_metadata(name, maturity)
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
        error_types: errors::ERROR_TYPES.to_vec(),
    }
}

#[cfg(test)]
mod tests;

mod signatures;
