mod backoff;
mod connection;
mod worker;

use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use serde_json::Value;

use self::connection::WriterSlot;
use self::worker::Worker;
use crate::error::PluginError;
use crate::host::{set_status, PluginHostEvent, PluginStatus};
use crate::manifest::LoadedPlugin;
use crate::protocol::{error_response_value, response_value, InvocationContext, RpcError};

pub(crate) enum WorkerCommand {
    Invoke {
        request: u64,
        command: String,
        args: Value,
        context: InvocationContext,
    },
    Restart,
    Shutdown,
}

pub(crate) struct WorkerHandle {
    pub(crate) commands: Sender<WorkerCommand>,
    writer: WriterSlot,
    status: Arc<Mutex<PluginStatus>>,
}

impl WorkerHandle {
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
        writer.send(message).map_err(|_| PluginError::ProcessExited)
    }

    pub(crate) fn status(&self) -> PluginStatus {
        self.status
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }
}

pub(crate) fn start_worker(
    plugin: LoadedPlugin,
    events: Sender<PluginHostEvent>,
    wake: Sender<()>,
) -> WorkerHandle {
    let (commands, receiver) = mpsc::channel();
    let writer = Arc::new(Mutex::new(None));
    let status = Arc::new(Mutex::new(PluginStatus::Stopped));
    let handle = WorkerHandle {
        commands,
        writer: Arc::clone(&writer),
        status: Arc::clone(&status),
    };
    let name = plugin.manifest.name.clone();
    let error_events = events.clone();
    let error_wake = wake.clone();
    let error_status = Arc::clone(&status);
    let spawn = thread::Builder::new()
        .name(format!("kawi-plugin-{name}"))
        .spawn(move || {
            Worker::new(plugin, events, wake, writer, status).run(receiver);
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
