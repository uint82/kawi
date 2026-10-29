use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use crate::keymap::{Binding, KeymapResolver};
use kawi_editor::BufferInfo;
use kawi_plugin::{
    Capability, CapabilityRegistry, InvocationContext, LoadedPlugin, PluginCatalog, PluginError,
    PluginHost, PluginHostEvent, PluginStatus, RpcError,
};
use kawi_watch::{Fingerprint, WatchService};
use serde_json::Value;

use super::coalesce::PendingBufferEvent;

mod capability;
mod support;

const MAX_LOG_LINES: usize = 200;
pub(crate) const MESSAGE_THROTTLE: Duration = Duration::from_secs(1);

#[derive(Debug, Default)]
pub(crate) struct DenialRecord {
    pub(super) count: u64,
}

#[derive(Debug)]
pub(super) struct DiagnosticWindow {
    pub(super) started: Instant,
    pub(super) count: u32,
}

#[derive(Default)]
pub(crate) struct PluginRuntime {
    pub(super) host: Option<PluginHost>,
    pub(super) events: Option<Receiver<PluginHostEvent>>,
    pub(super) catalog: PluginCatalog,
    pub(super) targets: Vec<PluginCommandTarget>,
    pub(super) bindings: Vec<Binding>,
    pub(super) logs: BTreeMap<String, VecDeque<String>>,
    pub(super) known_buffers: BTreeMap<u64, BufferInfo>,
    pub(super) pending: BTreeMap<(String, u64), PendingBufferEvent>,
    pub(super) last_delivered: BTreeMap<(String, u64), u64>,
    pub(super) last_mode: Option<(u64, String)>,
    pub(super) last_diagnostics_revision: u64,
    pub(super) resync_buffers: BTreeSet<(String, u64)>,
    pub(super) pending_activation_events: BTreeMap<String, VecDeque<(String, Value)>>,
    pub(super) permissions_path: Option<PathBuf>,
    pub(super) capabilities: CapabilityRegistry,
    pub(super) denials: BTreeMap<(String, Capability), DenialRecord>,
    pub(super) warnings: BTreeMap<String, Vec<String>>,
    pub(super) last_message: BTreeMap<String, Instant>,
    pub(super) throttled_messages: BTreeMap<String, u64>,
    pub(super) diagnostic_windows: BTreeMap<String, DiagnosticWindow>,
    pub(super) allow_once: BTreeSet<(String, Capability)>,
    pub(super) sandboxed: BTreeSet<String>,
    pub(super) permissions_watch: Option<WatchService>,
    pub(super) permissions_wake: Option<std::sync::mpsc::Sender<()>>,
    pub(super) last_permission_write: Option<Fingerprint>,
}

pub(crate) struct PluginCommandTarget {
    pub(super) plugin: String,
    pub(super) command: String,
}

impl PluginRuntime {
    pub(crate) fn target(&self, index: u32) -> Option<(&str, &str)> {
        self.targets
            .get(index as usize)
            .map(|target| (target.plugin.as_str(), target.command.as_str()))
    }

    pub(crate) fn invoke(
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

    pub(crate) fn respond(
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

    pub(crate) fn drain_events(&mut self) -> Vec<PluginHostEvent> {
        let mut events = Vec::new();
        if let Some(receiver) = self.events.as_ref() {
            while let Ok(event) = receiver.try_recv() {
                events.push(event);
            }
        }
        for event in &events {
            if let PluginHostEvent::Log { plugin, line } = event {
                self.push_log(plugin, line);
                if line.starts_with(kawi_plugin::NETWORK_WARNING_PREFIX) {
                    self.warnings
                        .entry(plugin.clone())
                        .or_default()
                        .push(line.clone());
                }
            }
        }
        events
    }

    pub(crate) fn plugins(&self) -> &[LoadedPlugin] {
        &self.catalog.plugins
    }

    pub(crate) fn apply_bindings(&self, keymaps: &mut KeymapResolver) {
        for binding in &self.bindings {
            keymaps.bind(binding.clone());
        }
    }

    pub(crate) fn status(&self, plugin: &str) -> Option<PluginStatus> {
        self.host.as_ref()?.status(plugin)
    }

    pub(crate) fn logs(&self, plugin: &str) -> Vec<String> {
        let mut lines = self
            .logs
            .get(plugin)
            .map(|lines| lines.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        for ((denied_plugin, capability), record) in &self.denials {
            if denied_plugin == plugin && record.count > 1 {
                lines.push(format!(
                    "denied {} calls: {}",
                    record.count,
                    capability.name()
                ));
            }
        }
        if let Some(dropped) = self.throttled_messages.get(plugin) {
            lines.push(format!("ui.message rate limited: {dropped} dropped"));
        }
        lines
    }

    pub(crate) fn restart(&self, plugin: Option<&str>) -> Result<(), PluginError> {
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

    pub(crate) fn shutdown(&self) {
        if let Some(host) = self.host.as_ref() {
            host.shutdown();
        }
    }

    pub(crate) fn retain_known_plugin_state(&mut self) {
        let known = self
            .catalog
            .plugins
            .iter()
            .map(|plugin| plugin.manifest.name.as_str())
            .collect::<BTreeSet<_>>();
        self.denials
            .retain(|(plugin, _), _| known.contains(plugin.as_str()));
        self.last_message
            .retain(|plugin, _| known.contains(plugin.as_str()));
        self.throttled_messages
            .retain(|plugin, _| known.contains(plugin.as_str()));
        self.diagnostic_windows
            .retain(|plugin, _| known.contains(plugin.as_str()));
        self.logs
            .retain(|plugin, _| known.contains(plugin.as_str()));
        self.warnings
            .retain(|plugin, _| known.contains(plugin.as_str()));
        self.allow_once
            .retain(|(plugin, _)| known.contains(plugin.as_str()));
        self.sandboxed
            .retain(|plugin| known.contains(plugin.as_str()));
    }

    pub(crate) fn push_log(&mut self, plugin: &str, line: &str) {
        let lines = self.logs.entry(plugin.to_string()).or_default();
        if lines.len() >= MAX_LOG_LINES {
            lines.pop_front();
        }
        lines.push_back(line.to_string());
    }
}
