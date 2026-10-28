mod coalesce;
mod commands;
mod diagnostics;
mod events;
mod host_methods;
mod invoke;
mod load;
mod notify;

use self::coalesce::PendingBufferEvent;

use std::collections::{BTreeMap, VecDeque};
use std::sync::mpsc::Receiver;

use crate::keymap::{Binding, KeymapResolver};
use kawi_editor::BufferInfo;
use kawi_plugin::{
    InvocationContext, LoadedPlugin, PluginCatalog, PluginError, PluginHost, PluginHostEvent,
    PluginStatus, RpcError,
};
use serde_json::Value;

#[cfg(test)]
mod tests;

const MAX_LOG_LINES: usize = 200;

#[derive(Default)]
pub(super) struct PluginRuntime {
    host: Option<PluginHost>,
    events: Option<Receiver<PluginHostEvent>>,
    catalog: PluginCatalog,
    targets: Vec<PluginCommandTarget>,
    bindings: Vec<Binding>,
    logs: BTreeMap<String, VecDeque<String>>,
    known_buffers: BTreeMap<u64, BufferInfo>,
    pending: BTreeMap<(String, u64), PendingBufferEvent>,
    last_delivered: BTreeMap<(String, u64), u64>,
    last_mode: Option<(u64, String)>,
    last_diagnostics_revision: u64,
    resync_buffers: std::collections::BTreeSet<(String, u64)>,
    pending_activation_events: BTreeMap<String, VecDeque<(String, Value)>>,
}

pub(super) struct PluginCommandTarget {
    plugin: String,
    command: String,
}

impl PluginRuntime {
    pub(super) fn target(&self, index: u32) -> Option<(&str, &str)> {
        self.targets
            .get(index as usize)
            .map(|target| (target.plugin.as_str(), target.command.as_str()))
    }

    pub(super) fn invoke(
        &self,
        target: u32,
        args: Value,
        context: InvocationContext,
    ) -> Result<(), PluginError> {
        let target = self
            .targets
            .get(target as usize)
            .ok_or_else(|| PluginError::Spawn("unknown plugin command".to_string()))?;
        let host = self
            .host
            .as_ref()
            .ok_or_else(|| PluginError::Spawn("plugin host is not running".to_string()))?;
        host.invoke(&target.plugin, &target.command, args, context)?;
        Ok(())
    }

    pub(super) fn respond(
        &self,
        plugin: &str,
        id: Value,
        result: Result<Value, RpcError>,
    ) -> Result<(), PluginError> {
        let host = self
            .host
            .as_ref()
            .ok_or_else(|| PluginError::Spawn("plugin host is not running".to_string()))?;
        host.respond(plugin, id, result)
    }

    pub(super) fn drain_events(&mut self) -> Vec<PluginHostEvent> {
        let mut events = Vec::new();
        if let Some(receiver) = self.events.as_ref() {
            while let Ok(event) = receiver.try_recv() {
                events.push(event);
            }
        }
        for event in &events {
            if let PluginHostEvent::Log { plugin, line } = event {
                self.push_log(plugin, line);
            }
        }
        events
    }

    pub(super) fn plugins(&self) -> &[LoadedPlugin] {
        &self.catalog.plugins
    }

    pub(super) fn apply_bindings(&self, keymaps: &mut KeymapResolver) {
        for binding in &self.bindings {
            keymaps.bind(binding.clone());
        }
    }

    pub(super) fn status(&self, plugin: &str) -> Option<PluginStatus> {
        self.host.as_ref()?.status(plugin)
    }

    pub(super) fn logs(&self, plugin: &str) -> Vec<String> {
        self.logs
            .get(plugin)
            .map(|lines| lines.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub(super) fn restart(&self, plugin: Option<&str>) -> Result<(), PluginError> {
        let Some(host) = self.host.as_ref() else {
            return Ok(());
        };
        match plugin {
            Some(plugin) => host.restart(plugin),
            None => {
                host.restart_all();
                Ok(())
            }
        }
    }

    pub(super) fn shutdown(&self) {
        if let Some(host) = self.host.as_ref() {
            host.shutdown();
        }
    }

    fn push_log(&mut self, plugin: &str, line: &str) {
        let lines = self.logs.entry(plugin.to_string()).or_default();
        if lines.len() >= MAX_LOG_LINES {
            lines.pop_front();
        }
        lines.push_back(line.to_string());
    }
}
