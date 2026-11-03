mod backoff;
mod connection;
mod sandbox;

pub(crate) use sandbox::policy as sandbox_policy;

#[cfg(feature = "wasm")]
pub(crate) use backoff::Backoff;
mod worker;

use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use serde_json::Value;

use self::connection::{Interaction, WriterSlot};
use self::worker::{Worker, WorkerContext};
use crate::capability::{Capability, CapabilityRegistry};
use crate::environment::HostEnvironment;
use crate::error::PluginError;
use crate::host::{set_status, PluginHostEvent, PluginStatus};
use crate::manifest::LoadedPlugin;
use crate::protocol::{error_response_value, response_value, InvocationContext, RpcError};

pub(crate) enum WorkerCommand {
    Activate,
    Invoke {
        request: u64,
        command: String,
        args: Value,
        context: Box<InvocationContext>,
    },
    Restart,
    Shutdown,
}

pub(crate) struct WorkerHandle {
    pub(crate) commands: Sender<WorkerCommand>,
    writer: WriterSlot,
    status: Arc<Mutex<PluginStatus>>,
    event_filter: Arc<Mutex<std::collections::BTreeSet<String>>>,
    restart: Arc<AtomicBool>,
    interaction: Arc<Interaction>,
    requirements: Arc<BTreeMap<String, Vec<Capability>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyOutcome {
    Queued,
    NotRunning,
    NotSubscribed,
    Denied,
    Overflow,
}

impl WorkerHandle {
    pub(crate) fn activate(&self) -> Result<(), PluginError> {
        self.commands
            .send(WorkerCommand::Activate)
            .map_err(|_| PluginError::ProcessExited)
    }

    pub(crate) fn notify_internal(&self, event: &str, payload: Value) -> NotifyOutcome {
        let Some(writer) = self.writer() else {
            return NotifyOutcome::NotRunning;
        };
        queue(&writer, crate::protocol::event_value(event, payload))
    }

    pub(crate) fn notify(&self, event: &str, payload: Value) -> NotifyOutcome {
        let Some(writer) = self.writer() else {
            return NotifyOutcome::NotRunning;
        };
        if !crate::protocol::EVENT_NAMES.contains(&event)
            || !self
                .event_filter
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .contains(event)
        {
            return NotifyOutcome::NotSubscribed;
        }
        queue(&writer, crate::protocol::event_value(event, payload))
    }

    fn writer(&self) -> Option<std::sync::mpsc::SyncSender<Value>> {
        self.writer
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub(crate) fn respond(
        &self,
        id: Value,
        result: Result<Value, RpcError>,
    ) -> Result<(), PluginError> {
        let interaction_id = id.as_u64();
        let message = match result {
            Ok(value) => response_value(id, value),
            Err(error) => error_response_value(id, &error),
        };
        let writer = self.writer().ok_or(PluginError::ProcessExited)?;
        let result = writer.try_send(message).map_err(|_| PluginError::QueueFull);
        if let Some(id) = interaction_id {
            self.interaction.end(id);
        }
        result
    }

    pub(crate) fn status(&self) -> PluginStatus {
        self.status
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub(crate) fn requirements(&self, command: &str) -> Option<&[Capability]> {
        self.requirements.get(command).map(Vec::as_slice)
    }

    pub(crate) fn restart(&self) -> Result<(), PluginError> {
        self.restart
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.commands
            .send(WorkerCommand::Restart)
            .map_err(|_| PluginError::ProcessExited)
    }
}

fn queue(writer: &std::sync::mpsc::SyncSender<Value>, message: Value) -> NotifyOutcome {
    match writer.try_send(message) {
        Ok(()) => NotifyOutcome::Queued,
        Err(std::sync::mpsc::TrySendError::Full(_)) => NotifyOutcome::Overflow,
        Err(std::sync::mpsc::TrySendError::Disconnected(_)) => NotifyOutcome::NotRunning,
    }
}

pub(crate) fn start_disabled_worker(
    plugin: LoadedPlugin,
    message: String,
    events: Sender<PluginHostEvent>,
    wake: Sender<()>,
) -> WorkerHandle {
    let (commands, receiver) = mpsc::channel();
    let status = Arc::new(Mutex::new(PluginStatus::Disabled));
    let requirements = Arc::new(plugin.command_requirements());
    let handle = WorkerHandle {
        commands,
        writer: Arc::new(Mutex::new(None)),
        status,
        event_filter: Arc::new(Mutex::new(std::collections::BTreeSet::new())),
        restart: Arc::new(AtomicBool::new(false)),
        interaction: Arc::new(Interaction::default()),
        requirements,
    };
    let name = plugin.manifest.name.clone();
    thread::spawn(move || {
        while let Ok(command) = receiver.recv() {
            match command {
                WorkerCommand::Invoke { request, .. } => crate::host::send_event(
                    &events,
                    &wake,
                    PluginHostEvent::InvocationFinished {
                        plugin: name.clone(),
                        request,
                        result: Err(PluginError::Initialize(message.clone())),
                    },
                ),
                WorkerCommand::Shutdown => break,
                WorkerCommand::Activate | WorkerCommand::Restart => {}
            }
        }
    });
    handle
}

pub(crate) fn start_worker(
    plugin: LoadedPlugin,
    capabilities: CapabilityRegistry,
    environment: HostEnvironment,
    events: Sender<PluginHostEvent>,
    wake: Sender<()>,
) -> WorkerHandle {
    let (commands, receiver) = mpsc::channel();
    let writer = Arc::new(Mutex::new(None));
    let status = Arc::new(Mutex::new(PluginStatus::Stopped));
    let event_filter = Arc::new(Mutex::new(std::collections::BTreeSet::new()));
    let restart = Arc::new(AtomicBool::new(false));
    let interaction = Arc::new(Interaction::default());
    let requirements = Arc::new(plugin.command_requirements());
    let handle = WorkerHandle {
        commands,
        writer: Arc::clone(&writer),
        status: Arc::clone(&status),
        event_filter: Arc::clone(&event_filter),
        restart: Arc::clone(&restart),
        interaction: Arc::clone(&interaction),
        requirements,
    };
    let name = plugin.manifest.name.clone();
    let error_events = events.clone();
    let error_wake = wake.clone();
    let error_status = Arc::clone(&status);
    let spawn = thread::Builder::new()
        .name(format!("kawi-plugin-{name}"))
        .spawn(move || {
            Worker::new(
                plugin,
                WorkerContext {
                    events,
                    wake,
                    writer,
                    status,
                    event_filter,
                    capabilities,
                    environment,
                    restart,
                    interaction,
                },
            )
            .run(receiver);
        });
    if let Err(error) = spawn {
        set_status(
            &error_status,
            &error_events,
            &error_wake,
            &name,
            PluginStatus::Error(error.to_string()),
        );
    }
    handle
}
