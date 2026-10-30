use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError};
use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::capability::Capability;
use crate::error::PluginError;
use crate::host::PluginStatus;
use crate::process::NotifyOutcome;
use crate::protocol::{InvocationContext, RpcError};

use super::super::state::{fail_pending, PendingCalls};

pub enum WasmCommand {
    Activate,
    Invoke {
        request: u64,
        command: String,
        args: Value,
        context: InvocationContext,
    },
    Notify {
        event: String,
        payload: Value,
    },
    Restart,
    Shutdown,
}

pub(crate) struct WasmWorkerHandle {
    pub commands: SyncSender<WasmCommand>,
    pub(super) status: Arc<Mutex<PluginStatus>>,
    pub(super) event_filter: Arc<Mutex<BTreeSet<String>>>,
    pub(super) pending: PendingCalls,
    pub(super) cancel: Arc<AtomicBool>,
    pub(super) requirements: Arc<BTreeMap<String, Vec<Capability>>>,
}

impl WasmWorkerHandle {
    pub(crate) fn activate(&self) -> Result<(), PluginError> {
        self.commands
            .try_send(WasmCommand::Activate)
            .map_err(|_| PluginError::ProcessExited)
    }

    pub(crate) fn notify(&self, event: &str, payload: Value) -> NotifyOutcome {
        if !self
            .event_filter
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .contains(event)
        {
            return NotifyOutcome::NotSubscribed;
        }
        match self.commands.try_send(WasmCommand::Notify {
            event: event.to_string(),
            payload,
        }) {
            Ok(()) => NotifyOutcome::Queued,
            Err(TrySendError::Full(_)) => NotifyOutcome::Overflow,
            Err(TrySendError::Disconnected(_)) => NotifyOutcome::NotRunning,
        }
    }

    pub(crate) fn respond(
        &self,
        id: Value,
        result: Result<Value, RpcError>,
    ) -> Result<(), PluginError> {
        let id = id
            .as_u64()
            .ok_or_else(|| PluginError::Protocol("invalid host request id".to_string()))?;
        let sender = self
            .pending
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(&id);
        if let Some(sender) = sender {
            let _ = sender.send(result);
        }
        Ok(())
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
        self.cancel_pending("plugin restarting");
        self.commands
            .try_send(WasmCommand::Restart)
            .map_err(|_| PluginError::ProcessExited)
    }

    pub(crate) fn shutdown(&self) {
        self.cancel_pending("plugin stopped");
        let _ = self.commands.send(WasmCommand::Shutdown);
    }

    fn cancel_pending(&self, reason: &str) {
        self.cancel.store(true, Ordering::SeqCst);
        fail_pending(
            &self.pending,
            RpcError::new(crate::protocol::PLUGIN_UNAVAILABLE, reason),
        );
    }

    pub(crate) fn invoke(
        &self,
        request: u64,
        command: String,
        args: Value,
        context: InvocationContext,
    ) -> Result<(), PluginError> {
        self.commands
            .try_send(WasmCommand::Invoke {
                request,
                command,
                args,
                context,
            })
            .map_err(|_| PluginError::QueueFull)
    }
}
