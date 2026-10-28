mod coalesce;
mod commands;
mod diagnostics;
mod events;
mod host_methods;
mod invoke;
mod load;
mod notify;

use self::coalesce::PendingBufferEvent;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use crate::keymap::{Binding, KeymapResolver};
use kawi_editor::BufferInfo;
use kawi_plugin::{
    load_permission_state, Capability, CapabilityRegistry, InvocationContext, LoadedPlugin,
    PluginCatalog, PluginError, PluginHost, PluginHostEvent, PluginStatus, RpcError, Scope,
};
use serde_json::Value;

#[cfg(test)]
mod tests;

const MAX_LOG_LINES: usize = 200;
const MESSAGE_THROTTLE: Duration = Duration::from_secs(1);

#[derive(Debug, Default)]
struct DenialRecord {
    count: u64,
}

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
    permissions_path: Option<PathBuf>,
    capabilities: CapabilityRegistry,
    denials: BTreeMap<(String, Capability), DenialRecord>,
    last_message: BTreeMap<String, Instant>,
    throttled_messages: BTreeMap<String, u64>,
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

    pub(super) fn request_scope(&self, plugin: &str, capability: Capability) -> Option<Scope> {
        self.catalog
            .plugins
            .iter()
            .find(|loaded| loaded.manifest.name == plugin)
            .and_then(|loaded| loaded.manifest.capability_requests.get(capability))
            .cloned()
    }

    pub(super) fn denied_capability(&self, plugin: &str, method: &str) -> Option<Capability> {
        match kawi_plugin::host_method_capability(method) {
            kawi_plugin::MethodCapability::Required(capability)
                if !self.capabilities.has(plugin, capability) =>
            {
                Some(capability)
            }
            _ => None,
        }
    }

    pub(super) fn capability_error(&self, plugin: &str, capability: Capability) -> RpcError {
        let scope = self
            .request_scope(plugin, capability)
            .filter(|scope| !matches!(scope, Scope::Unscoped))
            .map(|scope| Value::Object(scope.to_json_object()));
        RpcError::capability_denied(capability.name(), scope)
    }

    pub(super) fn denial_records(&self, plugin: &str) -> Vec<String> {
        let mut lines = Vec::new();
        for ((denied_plugin, capability), record) in &self.denials {
            if denied_plugin != plugin {
                continue;
            }
            lines.extend(denial_lines(
                plugin,
                *capability,
                self.request_scope(plugin, *capability),
            ));
            if record.count > 1 {
                lines.push(format!(
                    "denied {} calls: {}",
                    record.count,
                    capability.name()
                ));
            }
        }
        lines
    }

    pub(super) fn record_denial(&mut self, plugin: &str, capability: Capability) {
        if !self
            .catalog
            .plugins
            .iter()
            .any(|loaded| loaded.manifest.name == plugin)
        {
            return;
        }
        let first = self
            .denials
            .get(&(plugin.to_string(), capability))
            .is_none_or(|record| record.count == 0);
        if first {
            for line in denial_lines(plugin, capability, self.request_scope(plugin, capability)) {
                self.push_log(plugin, &line);
            }
        }
        self.denials
            .entry((plugin.to_string(), capability))
            .or_default()
            .count += 1;
    }

    pub(super) fn allow_message(&mut self, plugin: &str) -> bool {
        let now = Instant::now();
        if self
            .last_message
            .get(plugin)
            .is_some_and(|last| now.duration_since(*last) < MESSAGE_THROTTLE)
        {
            *self
                .throttled_messages
                .entry(plugin.to_string())
                .or_default() += 1;
            return false;
        }
        self.last_message.insert(plugin.to_string(), now);
        true
    }

    pub(super) fn reload_permissions(&mut self) -> Vec<String> {
        let Some(path) = self.permissions_path.as_deref() else {
            return Vec::new();
        };
        let state = load_permission_state(&self.catalog, path);
        self.capabilities.replace(state.grants);
        state.warnings
    }

    pub(super) fn capability_lines(&self, plugin: &str) -> Vec<String> {
        let effective = self.capabilities.get(plugin);
        let granted = describe_capabilities(effective.granted.keys().copied());
        let missing = describe_capabilities(effective.missing.iter().copied());
        let mut lines = vec![format!("granted: {granted}")];
        if missing != "none" {
            lines.push(format!("ungranted: {missing}"));
        }
        if effective.changed {
            lines.push("changed: request hash mismatch".to_string());
        }
        let stale = describe_capabilities(effective.stale.iter().copied());
        if stale != "none" {
            lines.push(format!("stale: {stale}"));
        }
        lines
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

    fn retain_known_plugin_state(&mut self) {
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
        self.logs
            .retain(|plugin, _| known.contains(plugin.as_str()));
    }

    fn push_log(&mut self, plugin: &str, line: &str) {
        let lines = self.logs.entry(plugin.to_string()).or_default();
        if lines.len() >= MAX_LOG_LINES {
            lines.pop_front();
        }
        lines.push_back(line.to_string());
    }
}

fn describe_capabilities(capabilities: impl Iterator<Item = Capability>) -> String {
    let names = capabilities
        .map(|capability| {
            if capability.is_advisory() {
                format!("{} (advisory)", capability.name())
            } else {
                capability.name().to_string()
            }
        })
        .collect::<Vec<_>>();
    if names.is_empty() {
        "none".to_string()
    } else {
        names.join(", ")
    }
}

fn denial_lines(plugin: &str, capability: Capability, scope: Option<Scope>) -> [String; 3] {
    [
        format!("denied {}", capability.name()),
        format!("add to permissions.toml: [plugins.{plugin}.grant]"),
        format!(
            "\"{}\" = {}",
            capability.name(),
            grant_value(scope.as_ref())
        ),
    ]
}

fn grant_value(scope: Option<&Scope>) -> String {
    match scope {
        None | Some(Scope::Unscoped) => "true".to_string(),
        Some(Scope::Paths(paths)) => format!("{{ paths = {paths:?} }}"),
        Some(Scope::Commands(commands)) => format!("{{ commands = {commands:?} }}"),
        Some(Scope::Hosts(hosts)) => format!("{{ hosts = {hosts:?} }}"),
    }
}
