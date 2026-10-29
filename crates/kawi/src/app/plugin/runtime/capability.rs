use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use kawi_plugin::{load_permission_state, Capability, RpcError, Scope};
use kawi_watch::{Fingerprint, WatchService};

use super::support::{changed_restart_plugins, denial_lines, describe_capabilities, same_path};
use super::{DiagnosticWindow, PluginRuntime, MESSAGE_THROTTLE};

const DIAGNOSTIC_RATE_WINDOW: Duration = Duration::from_secs(1);
const MAX_DIAGNOSTIC_PUBLISHES: u32 = 20;

impl PluginRuntime {
    pub(crate) fn mark_sandboxed(&mut self, plugin: &str) {
        self.sandboxed.insert(plugin.to_string());
    }

    fn is_sandboxed(&self, plugin: &str) -> bool {
        self.sandboxed.contains(plugin)
    }

    pub(crate) fn has_capability(&self, plugin: &str, capability: Capability) -> bool {
        self.capabilities.has(plugin, capability)
    }

    pub(crate) fn is_wasm(&self, plugin: &str) -> bool {
        self.catalog.plugins.iter().any(|loaded| {
            loaded.manifest.name == plugin
                && loaded.manifest.runtime == kawi_plugin::PluginRuntime::Wasm
        })
    }

    pub(crate) fn request_hash(&self, plugin: &str) -> Option<String> {
        self.catalog
            .plugins
            .iter()
            .find(|loaded| loaded.manifest.name == plugin)
            .map(|loaded| loaded.manifest.capability_requests.request_hash())
    }

    pub(crate) fn is_changed(&self, plugin: &str) -> bool {
        self.capabilities.get(plugin).changed
    }

    pub(crate) fn changed_requests(&self, plugin: &str) -> Vec<String> {
        self.capabilities.get(plugin).changed_requests.clone()
    }

    pub(crate) fn changed_capabilities(&self, plugin: &str) -> Vec<Capability> {
        self.capabilities.get(plugin).changed_capabilities.clone()
    }

    pub(crate) fn allow_next(&mut self, plugin: &str, capability: Capability) {
        self.allow_once.insert((plugin.to_string(), capability));
    }

    pub(crate) fn take_allow_next(&mut self, plugin: &str, capability: Capability) -> bool {
        self.allow_once.remove(&(plugin.to_string(), capability))
    }

    pub(crate) fn request_scope(&self, plugin: &str, capability: Capability) -> Option<Scope> {
        self.catalog
            .plugins
            .iter()
            .find(|loaded| loaded.manifest.name == plugin)
            .and_then(|loaded| loaded.manifest.capability_requests.get(capability))
            .cloned()
    }

    pub(crate) fn denied_capability(&self, plugin: &str, method: &str) -> Option<Capability> {
        match kawi_plugin::host_method_capability(method) {
            kawi_plugin::MethodCapability::Required(capability)
                if !self.capabilities.has(plugin, capability) =>
            {
                Some(capability)
            }
            _ => None,
        }
    }

    pub(crate) fn capability_error(&self, plugin: &str, capability: Capability) -> RpcError {
        let scope = self
            .request_scope(plugin, capability)
            .filter(|scope| !matches!(scope, Scope::Unscoped))
            .map(|scope| serde_json::Value::Object(scope.to_json_object()));
        RpcError::capability_denied(capability.name(), scope)
    }

    pub(crate) fn denial_records(&self, plugin: &str) -> Vec<String> {
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

    pub(crate) fn clear_denial(&mut self, plugin: &str, capability: Capability) {
        self.denials.remove(&(plugin.to_string(), capability));
    }

    pub(crate) fn record_denial(&mut self, plugin: &str, capability: Capability) {
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

    pub(crate) fn allow_diagnostic_publish(&mut self, plugin: &str) -> bool {
        let now = Instant::now();
        let window =
            self.diagnostic_windows
                .entry(plugin.to_string())
                .or_insert(DiagnosticWindow {
                    started: now,
                    count: 0,
                });
        if now.duration_since(window.started) >= DIAGNOSTIC_RATE_WINDOW {
            window.started = now;
            window.count = 0;
        }
        if window.count >= MAX_DIAGNOSTIC_PUBLISHES {
            return false;
        }
        window.count += 1;
        true
    }

    pub(crate) fn allow_message(&mut self, plugin: &str) -> bool {
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

    pub(crate) fn reload_permissions(&mut self) -> Vec<String> {
        let (warnings, restarted) = self.reload_permissions_without_restart();
        self.restart_plugins(restarted);
        warnings
    }

    pub(crate) fn reload_permissions_without_restart(&mut self) -> (Vec<String>, BTreeSet<String>) {
        let Some(path) = self.permissions_path.as_deref() else {
            return (Vec::new(), BTreeSet::new());
        };
        let previous = self.capabilities.snapshot();
        let state = load_permission_state(&self.catalog, path);
        let restarted = changed_restart_plugins(&self.catalog, &previous, &state.grants);
        self.capabilities.replace(state.grants);
        (state.warnings, restarted)
    }

    fn restart_plugins(&self, restarted: BTreeSet<String>) {
        for plugin in restarted {
            let _ = self.restart(Some(&plugin));
        }
    }

    pub(crate) fn poll_permission_changes(&mut self) -> Option<Vec<String>> {
        let path = self.permissions_path.clone()?;
        let paths = self.permissions_watch.as_ref()?.try_recv_paths();
        if !paths.iter().any(|changed| same_path(changed, &path)) {
            return None;
        }
        if self.own_permission_write(&path) {
            return None;
        }
        self.last_permission_write = None;
        self.allow_once.clear();
        Some(self.reload_permissions())
    }

    fn own_permission_write(&self, path: &std::path::Path) -> bool {
        let current = Fingerprint::read(path).ok();
        match (current.as_ref(), self.last_permission_write.as_ref()) {
            (Some(current), Some(own)) => own.matches(current),
            _ => false,
        }
    }

    pub(crate) fn watch_permissions(
        &mut self,
        permissions: &std::path::Path,
        wake: std::sync::mpsc::Sender<()>,
    ) {
        self.permissions_wake = Some(wake.clone());
        let Some(parent) = permissions.parent() else {
            return;
        };
        let Ok(watch) = WatchService::new(wake) else {
            return;
        };
        watch.watch_directories([parent]);
        self.permissions_watch = Some(watch);
    }

    pub(crate) fn remember_permission_write(&mut self, path: &std::path::Path) {
        self.last_permission_write = Fingerprint::read(path).ok();
        if self.permissions_watch.is_none() {
            if let Some(wake) = self.permissions_wake.clone() {
                self.watch_permissions(path, wake);
            }
        }
    }

    pub(crate) fn capability_lines(&self, plugin: &str) -> Vec<String> {
        let effective = self.capabilities.get(plugin);
        let wasm = cfg!(feature = "wasm") && self.is_wasm(plugin);
        let sandboxed = self.is_sandboxed(plugin);
        let granted = describe_capabilities(effective.granted.keys().copied(), wasm, sandboxed);
        let missing = describe_capabilities(effective.missing.iter().copied(), wasm, sandboxed);
        let mut lines = vec![format!("granted: {granted}")];
        if missing != "none" {
            lines.push(format!("ungranted: {missing}"));
        }
        if effective.changed {
            lines.push("changed: request hash mismatch".to_string());
        }
        let stale = describe_capabilities(effective.stale.iter().copied(), wasm, sandboxed);
        if stale != "none" {
            lines.push(format!("stale: {stale}"));
        }
        lines
    }
}
