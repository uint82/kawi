use serde_json::Value;

use crate::capability::Capability;
use crate::error::PluginError;
use crate::host::PluginStatus;
use crate::protocol::{InvocationContext, RpcError};
use crate::NotifyOutcome;

#[cfg(feature = "wasm")]
use crate::wasm::worker::WasmWorkerHandle;

use super::disabled::DisabledWorker;

pub(super) enum HostWorker {
    #[cfg(feature = "wasm")]
    Wasm(WasmWorkerHandle),
    Disabled(DisabledWorker),
}

impl HostWorker {
    pub(super) fn activate(&self) -> Result<(), PluginError> {
        match self {
            #[cfg(feature = "wasm")]
            Self::Wasm(worker) => worker.activate(),
            Self::Disabled(worker) => worker.activate(),
        }
    }

    pub(super) fn notify(&self, event: &str, payload: Value) -> NotifyOutcome {
        match self {
            #[cfg(feature = "wasm")]
            Self::Wasm(worker) => worker.notify(event, payload),
            Self::Disabled(worker) => worker.notify(event, payload),
        }
    }

    pub(super) fn callback(
        &self,
        registration: u64,
        event: &str,
        payload: Value,
    ) -> Result<NotifyOutcome, PluginError> {
        match self {
            #[cfg(feature = "wasm")]
            Self::Wasm(worker) => worker.callback(registration, event, payload),
            Self::Disabled(worker) => worker.callback(registration, event, payload),
        }
    }

    pub(super) fn timer_callback(
        &self,
        timer: u64,
        payload: Option<String>,
    ) -> Result<NotifyOutcome, PluginError> {
        match self {
            #[cfg(feature = "wasm")]
            Self::Wasm(worker) => worker.timer_callback(timer, payload),
            Self::Disabled(worker) => worker.timer_callback(timer, payload),
        }
    }

    pub(super) fn notify_internal(&self, event: &str, payload: Value) -> NotifyOutcome {
        match self {
            #[cfg(feature = "wasm")]
            Self::Wasm(_) => NotifyOutcome::NotRunning,
            Self::Disabled(worker) => worker.notify_internal(event, payload),
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
            #[cfg(feature = "wasm")]
            Self::Wasm(worker) => worker.invoke(request, command, args, context),
            Self::Disabled(worker) => worker.invoke(request, command, args, context),
        }
    }

    pub(super) fn respond(
        &self,
        id: Value,
        result: Result<Value, RpcError>,
    ) -> Result<(), PluginError> {
        match self {
            #[cfg(feature = "wasm")]
            Self::Wasm(worker) => worker.respond(id, result),
            Self::Disabled(worker) => worker.respond(id, result),
        }
    }

    pub(super) fn restart(&self) -> Result<(), PluginError> {
        match self {
            #[cfg(feature = "wasm")]
            Self::Wasm(worker) => worker.restart(),
            Self::Disabled(worker) => worker.restart(),
        }
    }

    pub(super) fn status(&self) -> PluginStatus {
        match self {
            #[cfg(feature = "wasm")]
            Self::Wasm(worker) => worker.status(),
            Self::Disabled(worker) => worker.status(),
        }
    }

    pub(super) fn requirements(&self, command: &str) -> Option<&[Capability]> {
        match self {
            #[cfg(feature = "wasm")]
            Self::Wasm(worker) => worker.requirements(command),
            Self::Disabled(worker) => worker.requirements(command),
        }
    }

    pub(super) fn shutdown(&self) {
        match self {
            #[cfg(feature = "wasm")]
            Self::Wasm(worker) => worker.shutdown(),
            Self::Disabled(worker) => worker.shutdown(),
        }
    }
}
