use std::time::{Duration, Instant};

use kawi_plugin::{Capability, RpcError, Scope};

use super::support::{denial_lines, describe_capabilities};
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
            lines.extend(denial_lines(*capability));
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

    pub(crate) fn record_denial(&mut self, plugin: &str, capability: Capability) -> bool {
        if !self
            .catalog
            .plugins
            .iter()
            .any(|loaded| loaded.manifest.name == plugin)
        {
            return false;
        }
        let first = self
            .denials
            .get(&(plugin.to_string(), capability))
            .is_none_or(|record| record.count == 0);
        if first {
            for line in denial_lines(capability) {
                self.push_log(plugin, &line);
            }
        }
        self.denials
            .entry((plugin.to_string(), capability))
            .or_default()
            .count += 1;
        first
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

    pub(crate) fn capability_lines(&self, plugin: &str) -> Vec<String> {
        let effective = self.capabilities.get(plugin);
        let wasm = cfg!(feature = "wasm") && self.is_wasm(plugin);
        let sandboxed = self.is_sandboxed(plugin);
        let declared = describe_capabilities(effective.granted.keys().copied(), wasm, sandboxed);
        let mut lines = vec![format!("declared: {declared}")];
        if let Some(warnings) = self.warnings.get(plugin) {
            lines.extend(
                warnings
                    .iter()
                    .map(|warning| super::super::text::sanitize(warning)),
            );
        }
        lines
    }
}
