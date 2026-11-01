use std::collections::BTreeSet;
use std::time::Duration;

use serde_json::Value;
use wasmtime::component::{HasSelf, Linker};
use wasmtime::{Store, UpdateDeadline};

use crate::capability::{Capability, CapabilityRegistry};
use crate::environment::HostEnvironment;
use crate::error::PluginError;
use crate::host::{send_event, PluginHostEvent};
use crate::manifest::LoadedPlugin;
use crate::protocol::PROTOCOL_VERSION;

use super::bindings::kawi::plugin::types::Argument;
use super::bindings::kawi::plugin::{
    config_read, diagnostics_publish, editor_execute, editor_read, editor_write, process_run,
    types, ui_message,
};
use super::bindings::Host_;
use super::convert::{argument_from_json, argument_to_json, event_from_json, rpc_from_wit_error};
use super::name_lookup;
use super::net::NetworkPolicy;
use super::policy::missing_import_capabilities;
use super::process::ProcessRunner;
use super::state::{output_pipes, HostBridge, PendingCalls, WasmState};
use super::wasi_setup::{build_wasi, WasiSetup};
use super::WasmRuntime;

const WALL_CLOCK_CHECK_TICKS: u64 = 1;
const MAX_COMPONENT_BYTES: u64 = 64 * 1024 * 1024;

pub(crate) struct Session {
    pub(crate) store: Store<WasmState>,
    pub(crate) instance: Host_,
}

impl Session {
    pub(crate) fn initialize(&mut self) -> Result<(), PluginError> {
        self.begin_call();
        let result = self.instance.kawi_plugin_lifecycle().call_init(
            &mut self.store,
            u32::try_from(PROTOCOL_VERSION).unwrap_or(u32::MAX),
        );
        let error = self.finish_call(result);
        error
            .map_err(|error| PluginError::Initialize(format!("wasm init: {error:#}")))?
            .map_err(|error| PluginError::Initialize(error.message))
    }

    pub(crate) fn invoke(&mut self, command: &str, args: Value) -> Result<Value, PluginError> {
        let args = wasm_arguments(args)?;
        self.begin_call();
        let result = self.instance.kawi_plugin_lifecycle().call_invoke_command(
            &mut self.store,
            command,
            &args,
        );
        let result = self.finish_call(result)?;
        match result {
            Ok(value) => argument_to_json(value),
            Err(error) => Err(rpc_from_wit_error(error)),
        }
    }

    pub(crate) fn event(&mut self, event: &str, payload: Value) -> Result<(), PluginError> {
        let event = event_from_json(event, payload).map_err(PluginError::Rpc)?;
        self.begin_call();
        let result = self
            .instance
            .kawi_plugin_lifecycle()
            .call_on_event(&mut self.store, &event);
        self.finish_call(result)
    }

    fn begin_call(&mut self) {
        self.store.set_epoch_deadline(WALL_CLOCK_CHECK_TICKS);
        self.store.data().begin_call();
    }

    fn finish_call<T>(&mut self, result: wasmtime::Result<T>) -> Result<T, PluginError> {
        let expired = self.store.data().call_expired();
        self.store.data_mut().clear_call();
        self.store.data_mut().drain_output();
        result.map_err(|error| {
            if expired {
                PluginError::Timeout
            } else {
                PluginError::Trap(format!("wasm trap: {error:#}"))
            }
        })
    }
}

pub(crate) fn add_kawi_interfaces(
    linker: &mut Linker<WasmState>,
    capabilities: &CapabilityRegistry,
    plugin: &str,
) -> Result<(), PluginError> {
    types::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
        .map_err(linker_error)?;
    config_read::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
        .map_err(linker_error)?;
    if capabilities.has(plugin, Capability::EditorRead) {
        editor_read::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::EditorWrite) {
        editor_write::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::EditorExecute) {
        editor_execute::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::UiMessage) {
        ui_message::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::DiagnosticsPublish) {
        diagnostics_publish::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::ProcessSpawn) {
        process_run::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    Ok(())
}

fn linker_error(error: wasmtime::Error) -> PluginError {
    PluginError::Initialize(format!("wasm linker: {error}"))
}

fn wasm_arguments(value: Value) -> Result<Vec<Argument>, PluginError> {
    let values = match value {
        Value::Null => Vec::new(),
        Value::Array(values) => values,
        value => vec![value],
    };
    values
        .into_iter()
        .map(|value| argument_from_json(value).map_err(PluginError::Rpc))
        .collect()
}

impl Session {
    pub(crate) fn open(
        plugin: &LoadedPlugin,
        capabilities: &CapabilityRegistry,
        environment: &HostEnvironment,
        events: std::sync::mpsc::Sender<PluginHostEvent>,
        wake: std::sync::mpsc::Sender<()>,
        pending: PendingCalls,
        cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<Self, PluginError> {
        let path = plugin
            .component_path()
            .ok_or_else(|| PluginError::Initialize("missing component path".to_string()))?;
        let metadata = std::fs::metadata(&path)
            .map_err(|error| PluginError::Initialize(format!("cannot read component: {error}")))?;
        if metadata.len() > MAX_COMPONENT_BYTES {
            return Err(PluginError::Initialize(
                "component exceeds the maximum size".to_string(),
            ));
        }
        let bytes = std::fs::read(&path)
            .map_err(|error| PluginError::Initialize(format!("cannot read component: {error}")))?;
        let runtime = WasmRuntime::new()?;
        let requested = plugin
            .manifest
            .capability_requests
            .iter()
            .map(|(capability, _)| capability)
            .collect::<BTreeSet<_>>();
        let requested_imports = requested.iter().copied().collect::<Vec<_>>();
        let (component, inspection) =
            runtime.inspect_component(&bytes, plugin.component_hash(), &requested_imports)?;
        let granted = Capability::ALL
            .into_iter()
            .filter(|capability| capabilities.has(&plugin.manifest.name, *capability))
            .collect::<BTreeSet<_>>();
        let missing = missing_import_capabilities(
            inspection.imports.iter().map(String::as_str),
            &requested,
            &granted,
        );
        if !missing.is_empty() {
            return Err(PluginError::MissingCapabilities(missing));
        }
        let mut linker = Linker::new(runtime.engine());
        add_kawi_interfaces(&mut linker, capabilities, &plugin.manifest.name)?;
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)
            .map_err(|error| PluginError::Initialize(format!("wasi linker: {error}")))?;
        super::connection_cap::add_to_linker(&mut linker)
            .map_err(|error| PluginError::Initialize(format!("wasi connection cap: {error}")))?;
        name_lookup::add_to_linker(&mut linker)
            .map_err(|error| PluginError::Initialize(format!("wasi name lookup: {error}")))?;
        let (stdout, stderr) = output_pipes();
        let network = NetworkPolicy::from_scope(
            capabilities
                .scope(&plugin.manifest.name, Capability::NetConnect)
                .as_ref(),
        );
        for warning in network.warnings() {
            send_event(
                &events,
                &wake,
                PluginHostEvent::Log {
                    plugin: plugin.manifest.name.clone(),
                    line: warning.clone(),
                },
            );
        }
        let mut warnings = Vec::new();
        let wasi = build_wasi(
            WasiSetup {
                plugin,
                capabilities,
                environment,
                network: network.clone(),
                stdout: stdout.clone(),
                stderr: stderr.clone(),
            },
            &mut warnings,
        )?;
        for warning in warnings {
            send_event(
                &events,
                &wake,
                PluginHostEvent::Log {
                    plugin: plugin.manifest.name.clone(),
                    line: warning,
                },
            );
        }
        let timeout = Duration::from_millis(plugin.manifest.timeout_ms);
        let bridge = HostBridge::new(
            plugin.manifest.name.clone(),
            events,
            wake,
            pending,
            cancel,
            timeout,
        );
        let process = ProcessRunner::new(plugin.clone(), capabilities.clone(), environment.clone());
        let state = WasmState::new(wasi, network, bridge, process, stdout, stderr);
        let mut store = Store::new(runtime.engine(), state);
        store.limiter(|state| &mut state.limits);
        store.epoch_deadline_callback(|context| {
            if context.data().call_expired() {
                Err(wasmtime::Error::msg(
                    "wasm call exceeded wall-clock timeout",
                ))
            } else {
                Ok(UpdateDeadline::Continue(WALL_CLOCK_CHECK_TICKS))
            }
        });
        store.data_mut().begin_call();
        store.set_epoch_deadline(WALL_CLOCK_CHECK_TICKS);
        let instance = Host_::instantiate(&mut store, &component, &linker)
            .map_err(|error| PluginError::Initialize(format!("wasm instantiate: {error}")))?;
        store.data_mut().clear_call();
        Ok(Self { store, instance })
    }
}
