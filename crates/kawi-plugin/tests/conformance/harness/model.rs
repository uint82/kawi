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
    pub(crate) stale_once: bool,
    pub(crate) invalid_position: bool,
    pub(crate) oversize_response: bool,
    pub(crate) withhold: bool,
    pub(crate) withhold_method: Option<String>,
    pub(crate) state_buffer: u64,
    pub(crate) state_buffer_after_stale: Option<u64>,
    pub(crate) state_text: Option<String>,
    pub(crate) deferred: Vec<DeferredResponse>,
}

pub(crate) struct HostPolicy {
    pub(crate) stale_revision: bool,
    pub(crate) invalid_position: bool,
    pub(crate) oversize_response: bool,
    pub(crate) withhold: bool,
    pub(crate) withhold_method: Option<String>,
    pub(crate) state_buffer: u64,
    pub(crate) state_text: Option<String>,
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
            stale_once: false,
            invalid_position: false,
            oversize_response: false,
            withhold: false,
            withhold_method: None,
            state_buffer: 1,
            state_buffer_after_stale: None,
            state_text: None,
            deferred: Vec::new(),
        }
    }

    pub(crate) fn take_host_policy(&mut self, method: &str) -> HostPolicy {
        let stale_revision = if method == "editor.apply_edit" && self.stale_once {
            self.stale_once = false;
            if let Some(buffer) = self.state_buffer_after_stale.take() {
                self.state_buffer = buffer;
            }
            true
        } else {
            self.stale_revision
        };
        HostPolicy {
            stale_revision,
            invalid_position: self.invalid_position,
            oversize_response: self.oversize_response,
            withhold: self.withhold,
            withhold_method: self.withhold_method.clone(),
            state_buffer: self.state_buffer,
            state_text: self.state_text.clone(),
        }
    }
}
