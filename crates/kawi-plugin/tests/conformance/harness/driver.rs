use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use serde_json::{json, Value};

use kawi_plugin::{
    host_method_capability, is_host_method_allowed, CapabilityRegistry, MethodCapability,
    PluginHost, PluginHostEvent, PluginStatus, RpcError,
};

use super::model::{DeferredResponse, Finished, HostRequestRecord, Observation, Policy};
use super::DRIVER_POLL;

pub(crate) struct Driver {
    pub(crate) host: Arc<PluginHost>,
    pub(crate) registry: CapabilityRegistry,
    pub(crate) observation: Arc<Mutex<Observation>>,
    pub(crate) policy: Arc<Mutex<Policy>>,
    pub(crate) stop: Arc<AtomicBool>,
    pub(crate) finished: Sender<Finished>,
}

impl Driver {
    pub(crate) fn spawn(self, events: Receiver<PluginHostEvent>) -> JoinHandle<()> {
        thread::Builder::new()
            .name("kawi-conformance-driver".to_string())
            .spawn(move || self.run(events))
            .expect("spawn conformance driver")
    }

    fn run(self, events: Receiver<PluginHostEvent>) {
        while !self.stop.load(Ordering::SeqCst) {
            let event = match events.recv_timeout(DRIVER_POLL) {
                Ok(event) => event,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            };
            self.handle(event);
        }
    }

    fn handle(&self, event: PluginHostEvent) {
        match event {
            PluginHostEvent::HostRequest {
                plugin,
                id,
                method,
                params,
            } => self.handle_host_request(&plugin, id, &method, params),
            PluginHostEvent::InvocationFinished {
                request, result, ..
            } => {
                let _ = self.finished.send(Finished { request, result });
            }
            PluginHostEvent::Log { line, .. } => self.observe_log(line),
            PluginHostEvent::StatusChanged { status, .. } => self.observe_status(status),
            PluginHostEvent::CapabilityRequired { capability, .. } => {
                self.observation
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .required
                    .push(capability);
            }
            PluginHostEvent::SandboxApplied { .. } => {}
        }
    }

    fn handle_host_request(&self, plugin: &str, id: Value, method: &str, params: Value) {
        let allowed = is_host_method_allowed(&self.registry, plugin, method);
        let index = self.record_host_request(method, !allowed);
        if !allowed {
            let _ = self
                .host
                .respond(plugin, id, Err(capability_denied(method)));
            self.mark_responded(index);
            return;
        }
        let (withhold, stale_revision, invalid_position) = self.policy_snapshot();
        let result = host_result(method, &params, stale_revision, invalid_position);
        if withhold {
            self.defer_response(plugin, id, result, index);
            return;
        }
        let _ = self.host.respond(plugin, id, result);
        self.mark_responded(index);
    }

    fn record_host_request(&self, method: &str, denied: bool) -> usize {
        let mut observation = self
            .observation
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        observation.host_requests.push(HostRequestRecord {
            method: method.to_string(),
            denied,
            responded: false,
        });
        observation.host_requests.len() - 1
    }

    fn mark_responded(&self, index: usize) {
        if let Some(request) = self
            .observation
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .host_requests
            .get_mut(index)
        {
            request.responded = true;
        }
    }

    fn policy_snapshot(&self) -> (bool, bool, bool) {
        let policy = self
            .policy
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        (
            policy.withhold,
            policy.stale_revision,
            policy.invalid_position,
        )
    }

    fn defer_response(
        &self,
        plugin: &str,
        id: Value,
        result: Result<Value, RpcError>,
        index: usize,
    ) {
        self.policy
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .deferred
            .push(DeferredResponse {
                plugin: plugin.to_string(),
                id,
                result,
                index,
            });
    }

    fn observe_log(&self, line: String) {
        self.observation
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .logs
            .push(line);
    }

    fn observe_status(&self, status: PluginStatus) {
        self.observation
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .statuses
            .push(status);
    }
}

fn capability_denied(method: &str) -> RpcError {
    match host_method_capability(method) {
        MethodCapability::Required(capability) => {
            RpcError::capability_denied(capability.name(), None)
        }
        _ => RpcError::capability_denied("unknown", None),
    }
}

fn host_result(
    method: &str,
    params: &Value,
    stale_revision: bool,
    invalid_position: bool,
) -> Result<Value, RpcError> {
    match method {
        "editor.get_state" => Ok(state_value()),
        "editor.apply_edit" if invalid_position => {
            Err(RpcError::invalid_position("edit position is out of range"))
        }
        "editor.apply_edit" if stale_revision => Err(RpcError::stale_revision(
            params
                .get("revision")
                .and_then(Value::as_u64)
                .unwrap_or_default(),
            7,
        )),
        "editor.apply_edit" => Ok(json!({ "revision": next_revision(params) })),
        "editor.set_diagnostics" | "ui.message" => Ok(Value::Null),
        _ => Ok(Value::Null),
    }
}

fn next_revision(params: &Value) -> u64 {
    params
        .get("revision")
        .and_then(Value::as_u64)
        .unwrap_or_default()
        .saturating_add(1)
}

fn state_value() -> Value {
    json!({
        "buffer": 1,
        "revision": 7,
        "path": "/tmp/conformance.rs",
        "language": "rust",
        "mode": "normal",
        "cursor": { "line": 0, "byte_column": 0 },
        "selection": null,
        "text": null,
    })
}
