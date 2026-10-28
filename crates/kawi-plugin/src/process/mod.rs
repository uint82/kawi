mod backoff;
mod connection;
mod sandbox;
mod worker;

use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use serde_json::Value;

use self::connection::WriterSlot;
use self::worker::{Worker, WorkerContext};
use crate::capability::CapabilityRegistry;
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

    pub(crate) fn notify(&self, event: &str, payload: Value) -> NotifyOutcome {
        let writer = self
            .writer
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        let Some(writer) = writer else {
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
        match writer.try_send(crate::protocol::event_value(event, payload)) {
            Ok(()) => NotifyOutcome::Queued,
            Err(std::sync::mpsc::TrySendError::Full(_)) => NotifyOutcome::Overflow,
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => NotifyOutcome::NotRunning,
        }
    }

    pub(crate) fn respond(
        &self,
        id: Value,
        result: Result<Value, RpcError>,
    ) -> Result<(), PluginError> {
        let message = match result {
            Ok(value) => response_value(id, value),
            Err(error) => error_response_value(id, &error),
        };
        let writer = self
            .writer
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
            .ok_or(PluginError::ProcessExited)?;
        writer.try_send(message).map_err(|_| PluginError::QueueFull)
    }

    pub(crate) fn status(&self) -> PluginStatus {
        self.status
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub(crate) fn restart(&self) -> Result<(), PluginError> {
        self.restart
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.commands
            .send(WorkerCommand::Restart)
            .map_err(|_| PluginError::ProcessExited)
    }
}

pub(crate) fn start_disabled_worker(
    plugin: LoadedPlugin,
    message: &'static str,
    events: Sender<PluginHostEvent>,
    wake: Sender<()>,
) -> WorkerHandle {
    let (commands, receiver) = mpsc::channel();
    let status = Arc::new(Mutex::new(PluginStatus::Disabled));
    let handle = WorkerHandle {
        commands,
        writer: Arc::new(Mutex::new(None)),
        status,
        event_filter: Arc::new(Mutex::new(std::collections::BTreeSet::new())),
        restart: Arc::new(AtomicBool::new(false)),
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
                        result: Err(PluginError::Initialize(message.to_string())),
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
    let handle = WorkerHandle {
        commands,
        writer: Arc::clone(&writer),
        status: Arc::clone(&status),
        event_filter: Arc::clone(&event_filter),
        restart: Arc::clone(&restart),
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
