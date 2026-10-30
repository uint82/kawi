use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::capability::{is_event_allowed, Capability, CapabilityRegistry};
use crate::environment::HostEnvironment;
use crate::error::PluginError;
use crate::payload_size::payload_too_large;
use crate::process::NotifyOutcome;
use crate::protocol::{InvocationContext, RpcError, INTERNAL_ERROR};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginStatus {
    Stopped,
    Starting,
    Ready,
    Backoff { remaining_ms: u64 },
    Disabled,
    Error(String),
}

#[derive(Debug)]
pub enum PluginHostEvent {
    InvocationFinished {
        plugin: String,
        request: u64,
        result: Result<Value, PluginError>,
    },
    HostRequest {
        plugin: String,
        id: Value,
        method: String,
        params: Value,
    },
    CapabilityRequired {
        plugin: String,
        capability: Capability,
    },
    Log {
        plugin: String,
        line: String,
    },
    StatusChanged {
        plugin: String,
        status: PluginStatus,
    },
    SandboxApplied {
        plugin: String,
    },
}

mod host_worker;
mod lifecycle;
mod wasm;

use self::host_worker::HostWorker;

pub struct PluginHost {
    workers: HashMap<String, HostWorker>,
    capabilities: CapabilityRegistry,
    next_request: AtomicU64,
    environment: HostEnvironment,
    events: Sender<PluginHostEvent>,
    wake: Sender<()>,
}

impl PluginHost {
    pub fn notify(
        &self,
        plugin: &str,
        event: &str,
        payload: Value,
    ) -> Result<NotifyOutcome, PluginError> {
        let worker = self
            .workers
            .get(plugin)
            .ok_or_else(|| PluginError::Spawn(format!("unknown plugin: {plugin}")))?;
        if !is_event_allowed(&self.capabilities, plugin, event) {
            return Ok(NotifyOutcome::Denied);
        }
        if payload_too_large(&payload) {
            return Ok(NotifyOutcome::Overflow);
        }
        Ok(worker.notify(event, payload))
    }

    pub fn activate(&self, plugin: &str) -> Result<(), PluginError> {
        let worker = self
            .workers
            .get(plugin)
            .ok_or_else(|| PluginError::Spawn(format!("unknown plugin: {plugin}")))?;
        worker.activate()
    }

    pub fn invoke(
        &self,
        plugin: &str,
        command: &str,
        args: Value,
        context: InvocationContext,
    ) -> Result<u64, PluginError> {
        let worker = self
            .workers
            .get(plugin)
            .ok_or_else(|| PluginError::Spawn(format!("unknown plugin: {plugin}")))?;
        if payload_too_large(&args) {
            return Err(PluginError::Protocol(
                "invocation payload exceeds the maximum size".to_string(),
            ));
        }
        if let Some(capability) = worker
            .requirements(command)
            .unwrap_or_default()
            .iter()
            .find(|capability| !self.capabilities.has(plugin, **capability))
        {
            return Err(PluginError::Rpc(RpcError::capability_denied(
                capability.name(),
                None,
            )));
        }
        let request = self.next_request.fetch_add(1, Ordering::Relaxed);
        worker.invoke(request, command.to_string(), args, context)?;
        Ok(request)
    }

    pub fn respond(
        &self,
        plugin: &str,
        id: Value,
        result: Result<Value, RpcError>,
    ) -> Result<(), PluginError> {
        let worker = self
            .workers
            .get(plugin)
            .ok_or_else(|| PluginError::Spawn(format!("unknown plugin: {plugin}")))?;
        let result = match result {
            Ok(value) if payload_too_large(&value) => Err(RpcError::new(
                INTERNAL_ERROR,
                "host response exceeds the maximum size",
            )),
            result => result,
        };
        worker.respond(id, result)
    }

    pub fn restart(&self, plugin: &str) -> Result<(), PluginError> {
        let worker = self
            .workers
            .get(plugin)
            .ok_or_else(|| PluginError::Spawn(format!("unknown plugin: {plugin}")))?;
        worker.restart()
    }

    pub fn restart_all(&self) {
        for worker in self.workers.values() {
            let _ = worker.restart();
        }
    }

    pub fn status(&self, plugin: &str) -> Option<PluginStatus> {
        self.workers.get(plugin).map(HostWorker::status)
    }

    pub fn plugins(&self) -> impl Iterator<Item = &str> {
        self.workers.keys().map(String::as_str)
    }

    pub fn shutdown(&self) {
        for worker in self.workers.values() {
            worker.shutdown();
        }
    }
}

impl Drop for PluginHost {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub(crate) fn send_event(
    events: &Sender<PluginHostEvent>,
    wake: &Sender<()>,
    event: PluginHostEvent,
) {
    if events.send(event).is_ok() {
        let _ = wake.send(());
    }
}

pub(crate) fn set_status(
    status: &Arc<Mutex<PluginStatus>>,
    events: &Sender<PluginHostEvent>,
    wake: &Sender<()>,
    plugin: &str,
    next: PluginStatus,
) {
    let changed = {
        let mut current = status.lock().unwrap_or_else(|error| error.into_inner());
        if *current == next {
            false
        } else {
            *current = next.clone();
            true
        }
    };
    if changed {
        send_event(
            events,
            wake,
            PluginHostEvent::StatusChanged {
                plugin: plugin.to_string(),
                status: next,
            },
        );
    }
}
