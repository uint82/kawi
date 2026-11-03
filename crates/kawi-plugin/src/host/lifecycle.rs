use std::collections::HashMap;
use std::sync::mpsc::Sender;

use crate::capability::CapabilityRegistry;
use crate::environment::HostEnvironment;
use crate::error::PluginError;
use crate::manifest::{LoadedPlugin, PluginCatalog, PluginRuntime};
use crate::process::start_worker;

use super::host_worker::HostWorker;
use super::wasm::wasm_worker;
use super::{PluginHost, PluginHostEvent};
use crate::jobs::JobManager;

impl PluginHost {
    pub fn start(
        catalog: &PluginCatalog,
        capabilities: CapabilityRegistry,
        environment: HostEnvironment,
        events: Sender<PluginHostEvent>,
        wake: Sender<()>,
    ) -> Self {
        let workers = build_workers(
            &catalog.plugins,
            &HostContext {
                capabilities: &capabilities,
                environment: &environment,
                events: &events,
                wake: &wake,
            },
        );
        let plugins = catalog
            .plugins
            .iter()
            .map(|plugin| (plugin.manifest.name.clone(), plugin.clone()))
            .collect();
        let jobs = JobManager::new(
            capabilities.clone(),
            environment.clone(),
            events.clone(),
            wake.clone(),
        );
        let host = Self {
            workers,
            plugins,
            jobs,
            capabilities,
            next_request: std::sync::atomic::AtomicU64::new(1),
            environment,
            events,
            wake,
        };
        host.activate_startup(catalog, None);
        host
    }

    pub fn reload(
        &mut self,
        catalog: &PluginCatalog,
        plugin: Option<&str>,
    ) -> Result<(), PluginError> {
        match plugin {
            Some(name) => self.reload_plugin(catalog, name),
            None => {
                self.jobs.shutdown();
                self.workers = build_workers(
                    &catalog.plugins,
                    &HostContext {
                        capabilities: &self.capabilities,
                        environment: &self.environment,
                        events: &self.events,
                        wake: &self.wake,
                    },
                );
                self.plugins = catalog
                    .plugins
                    .iter()
                    .map(|plugin| (plugin.manifest.name.clone(), plugin.clone()))
                    .collect();
                self.activate_startup(catalog, None);
                Ok(())
            }
        }
    }

    fn reload_plugin(&mut self, catalog: &PluginCatalog, name: &str) -> Result<(), PluginError> {
        let plugin = catalog
            .plugins
            .iter()
            .find(|plugin| plugin.manifest.name == name)
            .ok_or_else(|| PluginError::Spawn(format!("unknown plugin: {name}")))?;
        self.stop_jobs(name);
        if let Some(worker) = self.workers.remove(name) {
            worker.shutdown();
        }
        self.plugins.insert(name.to_string(), plugin.clone());
        let worker = build_worker(
            plugin,
            &HostContext {
                capabilities: &self.capabilities,
                environment: &self.environment,
                events: &self.events,
                wake: &self.wake,
            },
        );
        self.workers.insert(name.to_string(), worker);
        self.activate_startup(catalog, Some(name));
        Ok(())
    }

    fn activate_startup(&self, catalog: &PluginCatalog, plugin: Option<&str>) {
        for plugin in catalog
            .plugins
            .iter()
            .filter(|item| plugin.is_none_or(|name| item.manifest.name == name))
        {
            if plugin
                .manifest
                .activation_events
                .iter()
                .any(|event| event.event == "on_startup")
            {
                if let Some(worker) = self.workers.get(&plugin.manifest.name) {
                    let _ = worker.activate();
                }
            }
        }
    }
}

struct HostContext<'a> {
    capabilities: &'a CapabilityRegistry,
    environment: &'a HostEnvironment,
    events: &'a Sender<PluginHostEvent>,
    wake: &'a Sender<()>,
}

fn build_workers(
    plugins: &[LoadedPlugin],
    context: &HostContext<'_>,
) -> HashMap<String, HostWorker> {
    plugins
        .iter()
        .map(|plugin| {
            let name = plugin.manifest.name.clone();
            let worker = build_worker(plugin, context);
            (name, worker)
        })
        .collect()
}

fn build_worker(plugin: &LoadedPlugin, context: &HostContext<'_>) -> HostWorker {
    match plugin.manifest.runtime {
        PluginRuntime::Process => HostWorker::Process(start_worker(
            plugin.clone(),
            context.capabilities.clone(),
            context.environment.clone(),
            context.events.clone(),
            context.wake.clone(),
        )),
        PluginRuntime::Wasm => wasm_worker(
            plugin.clone(),
            context.capabilities,
            context.environment,
            context.events,
            context.wake,
        ),
    }
}
