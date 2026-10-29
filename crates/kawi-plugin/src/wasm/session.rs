use std::path::Path;

use serde_json::Value;
use wasmtime::component::{HasSelf, Linker};
use wasmtime::Store;
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;
use wasmtime_wasi::FsPerms;

use crate::capability::{Capability, CapabilityRegistry, Scope};
use crate::environment::HostEnvironment;
use crate::error::PluginError;
use crate::host::PluginHostEvent;
use crate::manifest::LoadedPlugin;
use crate::protocol::PROTOCOL_VERSION;

use super::bindings::kawi::plugin::types::Argument;
use super::bindings::kawi::plugin::{
    diagnostics_publish, editor_read, editor_write, process_run, types, ui_message,
};
use super::bindings::Host_;
use super::convert::{argument_from_json, argument_to_json, event_from_json, rpc_from_wit_error};
use super::process::ProcessRunner;
use super::state::{output_pipes, HostBridge, PendingCalls, WasmState};
use super::{WasiConfig, WasmRuntime};

pub(crate) const CALL_EPOCH_TICKS: u64 = 20;
const MAX_COMPONENT_BYTES: u64 = 64 * 1024 * 1024;

pub(crate) struct Session {
    pub(crate) store: Store<WasmState>,
    pub(crate) instance: Host_,
}

impl Session {
    pub(crate) fn initialize(&mut self) -> Result<(), PluginError> {
        self.store.set_epoch_deadline(CALL_EPOCH_TICKS);
        let result = self
            .instance
            .kawi_plugin_lifecycle()
            .call_init(
                &mut self.store,
                u32::try_from(PROTOCOL_VERSION).unwrap_or(u32::MAX),
            )
            .map_err(|error| PluginError::Initialize(format!("wasm init: {error}")))?;
        self.store.data_mut().drain_output();
        result.map_err(|error| PluginError::Initialize(error.message))
    }

    pub(crate) fn invoke(&mut self, command: &str, args: Value) -> Result<Value, PluginError> {
        self.store.set_epoch_deadline(CALL_EPOCH_TICKS);
        let args = wasm_arguments(args)?;
        let result = self
            .instance
            .kawi_plugin_lifecycle()
            .call_invoke_command(&mut self.store, command, &args)
            .map_err(|error| PluginError::Protocol(format!("wasm trap: {error}")))?;
        self.store.data_mut().drain_output();
        match result {
            Ok(value) => argument_to_json(value),
            Err(error) => Err(rpc_from_wit_error(error)),
        }
    }

    pub(crate) fn event(&mut self, event: &str, payload: Value) -> Result<(), PluginError> {
        self.store.set_epoch_deadline(CALL_EPOCH_TICKS);
        let event = event_from_json(event, payload).map_err(PluginError::Rpc)?;
        self.instance
            .kawi_plugin_lifecycle()
            .call_on_event(&mut self.store, &event)
            .map_err(|error| PluginError::Protocol(format!("wasm trap: {error}")))?;
        self.store.data_mut().drain_output();
        Ok(())
    }
}

pub(crate) fn add_kawi_interfaces(
    linker: &mut Linker<WasmState>,
    capabilities: &CapabilityRegistry,
    plugin: &str,
) -> Result<(), PluginError> {
    types::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
        .map_err(linker_error)?;
    if capabilities.has(plugin, Capability::EditorRead) {
        editor_read::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::EditorWrite) {
        editor_write::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
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
            .collect::<Vec<_>>();
        let (component, _) =
            runtime.inspect_component(&bytes, plugin.component_hash(), &requested)?;
        let mut linker = Linker::new(runtime.engine());
        add_kawi_interfaces(&mut linker, capabilities, &plugin.manifest.name)?;
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)
            .map_err(|error| PluginError::Initialize(format!("wasi linker: {error}")))?;
        let (stdout, stderr) = output_pipes();
        let wasi = build_wasi(plugin, capabilities, stdout.clone(), stderr.clone())?;
        let bridge = HostBridge::new(plugin.manifest.name.clone(), events, wake, pending);
        let process = ProcessRunner::new(plugin.clone(), capabilities.clone(), environment.clone());
        let state = WasmState::new(wasi, bridge, process, stdout, stderr);
        let mut store = Store::new(runtime.engine(), state);
        store.limiter(|state| &mut state.limits);
        store.set_epoch_deadline(CALL_EPOCH_TICKS);
        let instance = Host_::instantiate(&mut store, &component, &linker)
            .map_err(|error| PluginError::Initialize(format!("wasm instantiate: {error}")))?;
        Ok(Self { store, instance })
    }
}

fn build_wasi(
    plugin: &LoadedPlugin,
    capabilities: &CapabilityRegistry,
    stdout: MemoryOutputPipe,
    stderr: MemoryOutputPipe,
) -> Result<wasmtime_wasi::WasiCtx, PluginError> {
    let mut config = WasiConfig::new().stdout(stdout).stderr(stderr);
    config = add_preopens(
        config,
        plugin,
        capabilities,
        Capability::FsRead,
        FsPerms::ReadOnly,
    )?;
    config = add_preopens(
        config,
        plugin,
        capabilities,
        Capability::FsWrite,
        FsPerms::ReadWrite,
    )?;
    Ok(config.build())
}

fn add_preopens(
    mut config: WasiConfig,
    plugin: &LoadedPlugin,
    capabilities: &CapabilityRegistry,
    capability: Capability,
    perms: FsPerms,
) -> Result<WasiConfig, PluginError> {
    let Some(Scope::Paths(paths)) = capabilities.scope(&plugin.manifest.name, capability) else {
        return Ok(config);
    };
    for path in paths {
        config = config.preopen(Path::new(&path), &path, perms)?;
    }
    Ok(config)
}
