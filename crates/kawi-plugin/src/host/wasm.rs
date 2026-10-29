use std::sync::mpsc::Sender;

use crate::capability::CapabilityRegistry;
use crate::environment::HostEnvironment;
use crate::host::{send_event, PluginHostEvent};
use crate::manifest::LoadedPlugin;
use crate::process::start_disabled_worker;

use super::host_worker::HostWorker;

#[cfg(feature = "wasm")]
pub(super) fn wasm_worker(
    plugin: LoadedPlugin,
    capabilities: &CapabilityRegistry,
    environment: &HostEnvironment,
    events: &Sender<PluginHostEvent>,
    wake: &Sender<()>,
) -> HostWorker {
    if let Some(reason) = plugin.disabled_reason.clone() {
        send_event(
            events,
            wake,
            PluginHostEvent::Log {
                plugin: plugin.manifest.name.clone(),
                line: reason.clone(),
            },
        );
        return HostWorker::Disabled(start_disabled_worker(
            plugin,
            reason,
            events.clone(),
            wake.clone(),
        ));
    }
    HostWorker::Wasm(crate::wasm::worker::start_wasm_worker(
        plugin,
        capabilities.clone(),
        environment.clone(),
        events.clone(),
        wake.clone(),
    ))
}

#[cfg(not(feature = "wasm"))]
pub(super) fn wasm_worker(
    plugin: LoadedPlugin,
    _capabilities: &CapabilityRegistry,
    _environment: &HostEnvironment,
    events: &Sender<PluginHostEvent>,
    wake: &Sender<()>,
) -> HostWorker {
    let message = plugin
        .disabled_reason
        .clone()
        .unwrap_or_else(|| "wasm support is not enabled in this build".to_string());
    send_event(
        events,
        wake,
        PluginHostEvent::Log {
            plugin: plugin.manifest.name.clone(),
            line: message.clone(),
        },
    );
    HostWorker::Disabled(start_disabled_worker(
        plugin,
        message,
        events.clone(),
        wake.clone(),
    ))
}
