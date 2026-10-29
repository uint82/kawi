use crate::error::PluginError;
use crate::host::{send_event, PluginHostEvent};

use super::WasmWorker;

impl WasmWorker {
    pub(super) fn emit_capability_requirements(&self, error: &PluginError) {
        let PluginError::MissingCapabilities(capabilities) = error else {
            return;
        };
        for capability in capabilities {
            send_event(
                &self.events,
                &self.wake,
                PluginHostEvent::CapabilityRequired {
                    plugin: self.plugin.manifest.name.clone(),
                    capability: *capability,
                },
            );
        }
    }
}
