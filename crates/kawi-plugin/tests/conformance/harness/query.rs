#[cfg(feature = "wasm")]
use std::collections::BTreeSet;
use std::thread;
use std::time::{Duration, Instant};

use kawi_plugin::PluginStatus;
#[cfg(feature = "wasm")]
use kawi_plugin::{advertised_host_methods, host_method_capability, Capability, MethodCapability};

use super::host::ConformanceHost;
use super::model::Observation;
use super::{START_TIMEOUT, STATUS_POLL};

impl ConformanceHost {
    pub(crate) fn status(&self) -> Option<PluginStatus> {
        self.host.status(super::PLUGIN_NAME)
    }

    pub(crate) fn wait_backoff(&self) {
        if let Some(PluginStatus::Backoff { remaining_ms }) = self.status() {
            thread::sleep(Duration::from_millis(remaining_ms.saturating_add(10)));
        }
    }

    pub(crate) fn wait_for_host_request(&self, method: &str) -> bool {
        let deadline = Instant::now() + START_TIMEOUT;
        loop {
            if self
                .observation()
                .host_requests
                .iter()
                .any(|request| request.method == method)
            {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(STATUS_POLL);
        }
    }

    pub(crate) fn wait_for_host_response(&self, method: &str) -> bool {
        let deadline = Instant::now() + START_TIMEOUT;
        loop {
            if self
                .observation()
                .host_requests
                .iter()
                .any(|request| request.method == method && request.responded)
            {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(STATUS_POLL);
        }
    }

    pub(crate) fn observation(&self) -> Observation {
        self.observation
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub(crate) fn host_request_count(&self) -> usize {
        self.observation().host_requests.len()
    }

    pub(crate) fn last_host_request(&self, method: &str) -> Option<serde_json::Value> {
        self.observation()
            .host_requests
            .iter()
            .rev()
            .find(|request| request.method == method)
            .map(|request| request.params.clone())
    }

    #[cfg(feature = "wasm")]
    pub(crate) fn advertised_capabilities(&self) -> BTreeSet<Capability> {
        advertised_host_methods(&self.registry, super::PLUGIN_NAME)
            .into_iter()
            .filter_map(|method| match host_method_capability(method) {
                MethodCapability::Required(capability) => Some(capability),
                _ => None,
            })
            .collect()
    }

    #[cfg(feature = "wasm")]
    pub(crate) fn linked_capabilities(&self) -> BTreeSet<Capability> {
        let path = self
            .root
            .join("plugins")
            .join(super::PLUGIN_NAME)
            .join("plugin.wasm");
        let runtime = kawi_plugin::WasmRuntime::new().expect("wasm runtime");
        let inspection = runtime
            .inspect(&path, None, self.behavior.requests())
            .expect("inspect component");
        inspection.requested.into_iter().collect()
    }
}
