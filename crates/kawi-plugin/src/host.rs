use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::capability::{is_event_allowed, CapabilityRegistry};
use crate::environment::HostEnvironment;
use crate::error::PluginError;
use crate::manifest::{PluginCatalog, PluginRuntime};
use crate::process::{
    start_disabled_worker, start_worker, NotifyOutcome, WorkerCommand, WorkerHandle,
};
use crate::protocol::{InvocationContext, RpcError};

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

pub struct PluginHost {
    workers: HashMap<String, WorkerHandle>,
    capabilities: CapabilityRegistry,
    next_request: AtomicU64,
}

impl PluginHost {
    pub fn start(
        catalog: &PluginCatalog,
        capabilities: CapabilityRegistry,
        environment: HostEnvironment,
        events: Sender<PluginHostEvent>,
        wake: Sender<()>,
    ) -> Self {
        let workers: HashMap<String, WorkerHandle> = catalog
            .plugins
            .iter()
            .cloned()
            .map(|plugin| {
                let name = plugin.manifest.name.clone();
                let worker = match plugin.manifest.runtime {
                    PluginRuntime::Process => start_worker(
                        plugin,
                        capabilities.clone(),
                        environment.clone(),
                        events.clone(),
                        wake.clone(),
                    ),
                    PluginRuntime::Wasm => {
                        let message = plugin
                            .disabled_reason
                            .clone()
                            .unwrap_or_else(|| wasm_unavailable_message().to_string());
                        start_disabled_worker(plugin, message, events.clone(), wake.clone())
                    }
                };
                (name, worker)
            })
            .collect();
        for plugin in &catalog.plugins {
            if plugin.manifest.runtime == PluginRuntime::Wasm {
                send_event(
                    &events,
                    &wake,
                    PluginHostEvent::Log {
                        plugin: plugin.manifest.name.clone(),
                        line: plugin
                            .disabled_reason
                            .clone()
                            .unwrap_or_else(|| wasm_unavailable_message().to_string()),
                    },
                );
                continue;
            }
            if plugin
                .manifest
                .activation_events
                .iter()
                .any(|event| event.event == "on_startup")
            {
                if let Some(worker) = workers.get(&plugin.manifest.name) {
                    let _ = worker.activate();
                }
            }
        }
        Self {
            workers,
            capabilities,
            next_request: AtomicU64::new(1),
        }
    }

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
        let request = self.next_request.fetch_add(1, Ordering::Relaxed);
        worker
            .commands
            .send(WorkerCommand::Invoke {
                request,
                command: command.to_string(),
                args,
                context: Box::new(context),
            })
            .map_err(|_| PluginError::ProcessExited)?;
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
        self.workers.get(plugin).map(WorkerHandle::status)
    }

    pub fn plugins(&self) -> impl Iterator<Item = &str> {
        self.workers.keys().map(String::as_str)
    }

    pub fn shutdown(&self) {
        for worker in self.workers.values() {
            let _ = worker.commands.send(WorkerCommand::Shutdown);
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

fn wasm_unavailable_message() -> &'static str {
    #[cfg(feature = "wasm")]
    {
        "wasm execution is not available in this build phase"
    }
    #[cfg(not(feature = "wasm"))]
    {
        "wasm support is not enabled in this build"
    }
}
