use std::collections::BTreeMap;
use std::sync::mpsc::Sender;

use serde_json::Value;

use crate::capability::Capability;
use crate::error::PluginError;
use crate::host::{send_event, PluginHostEvent, PluginStatus};
use crate::manifest::LoadedPlugin;
use crate::protocol::{InvocationContext, RpcError};
use crate::NotifyOutcome;

pub(super) struct DisabledWorker {
    plugin: String,
    message: String,
    requirements: BTreeMap<String, Vec<Capability>>,
    events: Sender<PluginHostEvent>,
    wake: Sender<()>,
}

impl DisabledWorker {
    pub(super) fn new(
        plugin: LoadedPlugin,
        message: String,
        events: Sender<PluginHostEvent>,
        wake: Sender<()>,
    ) -> Self {
        Self {
            plugin: plugin.manifest.name.clone(),
            requirements: plugin.command_requirements(),
            message,
            events,
            wake,
        }
    }

    pub(super) fn activate(&self) -> Result<(), PluginError> {
        Ok(())
    }

    pub(super) fn notify(&self, _event: &str, _payload: Value) -> NotifyOutcome {
        NotifyOutcome::NotRunning
    }

    pub(super) fn callback(
        &self,
        _registration: u64,
        _event: &str,
        _payload: Value,
    ) -> Result<NotifyOutcome, PluginError> {
        Ok(NotifyOutcome::Denied)
    }

    pub(super) fn timer_callback(
        &self,
        _timer: u64,
        _payload: Option<String>,
    ) -> Result<NotifyOutcome, PluginError> {
        Ok(NotifyOutcome::Denied)
    }

    pub(super) fn notify_internal(&self, _event: &str, _payload: Value) -> NotifyOutcome {
        NotifyOutcome::NotRunning
    }

    pub(super) fn invoke(
        &self,
        request: u64,
        _command: String,
        _args: Value,
        _context: InvocationContext,
    ) -> Result<(), PluginError> {
        send_event(
            &self.events,
            &self.wake,
            PluginHostEvent::InvocationFinished {
                plugin: self.plugin.clone(),
                request,
                result: Err(PluginError::Initialize(self.message.clone())),
            },
        );
        Ok(())
    }

    pub(super) fn respond(
        &self,
        _id: Value,
        _result: Result<Value, RpcError>,
    ) -> Result<(), PluginError> {
        Ok(())
    }

    pub(super) fn restart(&self) -> Result<(), PluginError> {
        Ok(())
    }

    pub(super) fn status(&self) -> PluginStatus {
        PluginStatus::Disabled
    }

    pub(super) fn requirements(&self, command: &str) -> Option<&[Capability]> {
        self.requirements.get(command).map(Vec::as_slice)
    }

    pub(super) fn shutdown(&self) {}
}
