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
    pub(crate) denied: bool,
    pub(crate) responded: bool,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct Observation {
    pub(crate) host_requests: Vec<HostRequestRecord>,
    pub(crate) logs: Vec<String>,
    pub(crate) statuses: Vec<PluginStatus>,
    pub(crate) required: Vec<kawi_plugin::Capability>,
}

#[derive(Debug, Clone)]
pub(crate) struct Finished {
    pub(crate) request: u64,
    pub(crate) result: Result<Value, PluginError>,
}

pub(crate) struct Policy {
    pub(crate) stale_revision: bool,
    pub(crate) invalid_position: bool,
    pub(crate) withhold: bool,
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
            withhold: false,
            deferred: Vec::new(),
        }
    }
}
