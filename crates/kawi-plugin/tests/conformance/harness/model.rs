use serde_json::Value;

use kawi_plugin::{PluginError, PluginStatus, RpcError};

use super::super::scenario::Behavior;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Runtime {
    Process,
    Wasm,
}

impl Runtime {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Process => "process",
            Self::Wasm => "wasm",
        }
    }

    pub(crate) fn command(self, behavior: Behavior) -> &'static str {
        match self {
            Self::Process => behavior.process_command(),
            Self::Wasm => behavior.command(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HostRequestRecord {
    pub(crate) method: String,
    pub(crate) params: Value,
    pub(crate) denied: bool,
    pub(crate) responded: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct Observation {
    pub(crate) host_requests: Vec<HostRequestRecord>,
    pub(crate) logs: Vec<String>,
    pub(crate) statuses: Vec<PluginStatus>,
    pub(crate) required: Vec<kawi_plugin::Capability>,
    pub(crate) buffer: String,
    pub(crate) applied_edits: usize,
}

impl Default for Observation {
    fn default() -> Self {
        Self {
            host_requests: Vec::new(),
            logs: Vec::new(),
            statuses: Vec::new(),
            required: Vec::new(),
            buffer: "é\n".to_string(),
            applied_edits: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Finished {
    pub(crate) request: u64,
    pub(crate) result: Result<Value, PluginError>,
}

pub(crate) struct Policy {
    pub(crate) stale_revision: bool,
    pub(crate) invalid_position: bool,
    pub(crate) oversize_response: bool,
    pub(crate) withhold: bool,
    pub(crate) withhold_method: Option<String>,
    pub(crate) deferred: Vec<DeferredResponse>,
}

pub(crate) struct DeferredResponse {
    pub(crate) plugin: String,
    pub(crate) id: Value,
    pub(crate) result: Result<Value, RpcError>,
    pub(crate) index: usize,
}

impl Policy {
    pub(crate) fn new() -> Self {
        Self {
            stale_revision: false,
            invalid_position: false,
            oversize_response: false,
            withhold: false,
            withhold_method: None,
            deferred: Vec::new(),
        }
    }
}
