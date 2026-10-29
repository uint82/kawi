use serde_json::Value;

use crate::error::PluginError;
use crate::host::PluginStatus;
use crate::process::{NotifyOutcome, WorkerHandle};
use crate::protocol::{InvocationContext, RpcError};

#[cfg(feature = "wasm")]
use crate::wasm::worker::WasmWorkerHandle;

pub(super) enum HostWorker {
    Process(WorkerHandle),
    #[cfg(feature = "wasm")]
    Wasm(WasmWorkerHandle),
    Disabled(WorkerHandle),
}

impl HostWorker {
    pub(super) fn activate(&self) -> Result<(), PluginError> {
        match self {
            Self::Process(worker) => worker.activate(),
            #[cfg(feature = "wasm")]
            Self::Wasm(worker) => worker.activate(),
            Self::Disabled(_) => Ok(()),
        }
    }

    pub(super) fn notify(&self, event: &str, payload: Value) -> NotifyOutcome {
        match self {
            Self::Process(worker) => worker.notify(event, payload),
            #[cfg(feature = "wasm")]
            Self::Wasm(worker) => worker.notify(event, payload),
            Self::Disabled(_) => NotifyOutcome::NotRunning,
        }
    }

    pub(super) fn invoke(
        &self,
        request: u64,
        command: String,
        args: Value,
        context: InvocationContext,
    ) -> Result<(), PluginError> {
        match self {
            Self::Process(worker) => worker
                .commands
                .send(crate::process::WorkerCommand::Invoke {
                    request,
                    command,
                    args,
                    context: Box::new(context),
                })
                .map_err(|_| PluginError::ProcessExited),
            #[cfg(feature = "wasm")]
            Self::Wasm(worker) => worker.invoke(request, command, args, context),
            Self::Disabled(worker) => worker
                .commands
                .send(crate::process::WorkerCommand::Invoke {
                    request,
                    command,
                    args,
                    context: Box::new(context),
                })
                .map_err(|_| PluginError::ProcessExited),
        }
    }

    pub(super) fn respond(
        &self,
        id: Value,
        result: Result<Value, RpcError>,
    ) -> Result<(), PluginError> {
        match self {
            Self::Process(worker) | Self::Disabled(worker) => worker.respond(id, result),
            #[cfg(feature = "wasm")]
            Self::Wasm(worker) => worker.respond(id, result),
        }
    }

    pub(super) fn restart(&self) -> Result<(), PluginError> {
        match self {
            Self::Process(worker) | Self::Disabled(worker) => worker.restart(),
            #[cfg(feature = "wasm")]
            Self::Wasm(worker) => worker.restart(),
        }
    }

    pub(super) fn status(&self) -> PluginStatus {
        match self {
            Self::Process(worker) | Self::Disabled(worker) => worker.status(),
            #[cfg(feature = "wasm")]
            Self::Wasm(worker) => worker.status(),
        }
    }

    pub(super) fn shutdown(&self) {
        match self {
            Self::Process(worker) | Self::Disabled(worker) => {
                let _ = worker
                    .commands
                    .send(crate::process::WorkerCommand::Shutdown);
            }
            #[cfg(feature = "wasm")]
            Self::Wasm(worker) => worker.shutdown(),
        }
    }
}
