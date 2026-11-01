use wasmtime::component::ResourceTable;
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;
use wasmtime_wasi::sockets::WasiSocketsView;
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

use crate::protocol::{
    ApplyEditParams, ApplyEditResult, ConfigGetParams, ConfigGetResult, ConfigListResult,
    EditorExecuteParams, GetStateParams, RpcError, SetDiagnosticsParams, UiMessageParams,
    PLUGIN_UNAVAILABLE,
};

pub use super::bridge::{fail_pending, output_pipes, pending_calls, HostBridge, PendingCalls};

use super::bindings::kawi::plugin::types::{
    CommandInfo, Edit, Error, ProcessOutput, Setting, SettingValue, State,
};
use super::bindings::kawi::plugin::{
    config_read, diagnostics_publish, editor_execute, editor_read, editor_write, process_run,
    types, ui_message,
};
use super::byte_budget::NetworkBudget;
use super::convert::{
    command_infos_from_json, diagnostics_to_protocol, edits_to_protocol, error_from_rpc,
    setting_value_from_json, settings_from_json, state_from_json,
};
use super::limits::WasmLimits;
use super::net::NetworkPolicy;
use super::process::ProcessRunner;

pub struct WasmState {
    pub wasi: WasiCtx,
    pub table: ResourceTable,
    pub bridge: HostBridge,
    pub process: ProcessRunner,
    pub network: NetworkPolicy,
    pub network_budget: NetworkBudget,
    pub limits: WasmLimits,
    stdout: MemoryOutputPipe,
    stderr: MemoryOutputPipe,
    stdout_offset: usize,
    stderr_offset: usize,
}

impl WasmState {
    pub fn new(
        wasi: WasiCtx,
        network: NetworkPolicy,
        bridge: HostBridge,
        process: ProcessRunner,
        stdout: MemoryOutputPipe,
        stderr: MemoryOutputPipe,
    ) -> Self {
        Self {
            wasi,
            table: ResourceTable::new(),
            bridge,
            process,
            network,
            network_budget: NetworkBudget::new(),
            limits: WasmLimits::new(),
            stdout,
            stderr,
            stdout_offset: 0,
            stderr_offset: 0,
        }
    }

    pub fn begin_call(&self) {
        self.bridge.begin_call();
    }

    pub fn clear_call(&self) {
        self.bridge.clear_call();
    }

    pub fn call_expired(&self) -> bool {
        self.bridge.call_expired()
    }

    pub(crate) fn tracked_sockets(&mut self) -> super::connection_cap::TrackedSocketsView<'_> {
        let policy = self.network.clone();
        let deadline = self.bridge.deadline();
        let budget = self.network_budget.clone();
        let sockets = <Self as WasiSocketsView>::sockets(self);
        super::connection_cap::TrackedSocketsView::new(sockets, policy, deadline, budget)
    }

    pub fn drain_output(&mut self) {
        self.stdout_offset = drain(&self.stdout, self.stdout_offset, "stdout", &self.bridge);
        self.stderr_offset = drain(&self.stderr, self.stderr_offset, "stderr", &self.bridge);
    }
}

impl WasiView for WasmState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl types::Host for WasmState {}

impl editor_read::Host for WasmState {
    fn get_state(&mut self, include_text: bool) -> Result<State, Error> {
        let params = serialize_params(GetStateParams { include_text })?;
        let value = self
            .bridge
            .request("editor.get_state", params)
            .map_err(error_from_rpc)?;
        state_from_json(value).map_err(error_from_rpc)
    }
}

impl editor_write::Host for WasmState {
    fn apply_edit(&mut self, buffer: u64, revision: u64, edits: Vec<Edit>) -> Result<u64, Error> {
        let edits = edits_to_protocol(edits).map_err(error_from_rpc)?;
        let params = serialize_params(ApplyEditParams {
            buffer,
            revision,
            edits,
        })?;
        let value = self
            .bridge
            .request("editor.apply_edit", params)
            .map_err(error_from_rpc)?;
        let result: ApplyEditResult = serde_json::from_value(value).map_err(|error| {
            error_from_rpc(RpcError::new(PLUGIN_UNAVAILABLE, error.to_string()))
        })?;
        Ok(result.revision)
    }
}

impl editor_execute::Host for WasmState {
    fn execute(&mut self, command: String) -> Result<(), Error> {
        let params = serialize_params(EditorExecuteParams { command })?;
        self.bridge
            .request("editor.execute", params)
            .map(|_| ())
            .map_err(error_from_rpc)
    }

    fn list_commands(&mut self) -> Result<Vec<CommandInfo>, Error> {
        let value = self
            .bridge
            .request("editor.list_commands", serde_json::Value::Null)
            .map_err(error_from_rpc)?;
        command_infos_from_json(value).map_err(error_from_rpc)
    }
}

impl config_read::Host for WasmState {
    fn get(&mut self, key: String) -> Result<Option<SettingValue>, Error> {
        let params = serialize_params(ConfigGetParams { key })?;
        let value = self
            .bridge
            .request("config.get", params)
            .map_err(error_from_rpc)?;
        let result: ConfigGetResult = serde_json::from_value(value).map_err(|error| {
            error_from_rpc(RpcError::new(PLUGIN_UNAVAILABLE, error.to_string()))
        })?;
        result
            .value
            .map(|value| setting_value_from_json(value).map_err(error_from_rpc))
            .transpose()
    }

    fn get_all(&mut self) -> Result<Vec<Setting>, Error> {
        let value = self
            .bridge
            .request("config.list", serde_json::Value::Null)
            .map_err(error_from_rpc)?;
        let result: ConfigListResult = serde_json::from_value(value).map_err(|error| {
            error_from_rpc(RpcError::new(PLUGIN_UNAVAILABLE, error.to_string()))
        })?;
        settings_from_json(result.settings).map_err(error_from_rpc)
    }
}

impl ui_message::Host for WasmState {
    fn message(&mut self, text: String) -> Result<(), Error> {
        let params = serialize_params(UiMessageParams { message: text })?;
        self.bridge
            .request("ui.message", params)
            .map(|_| ())
            .map_err(error_from_rpc)
    }
}

impl diagnostics_publish::Host for WasmState {
    fn set(
        &mut self,
        _source: String,
        buffer: u64,
        revision: u64,
        items: Vec<super::bindings::kawi::plugin::types::Diagnostic>,
    ) -> Result<(), Error> {
        let params = serialize_params(SetDiagnosticsParams {
            buffer,
            revision,
            diagnostics: diagnostics_to_protocol(items),
        })?;
        let value = self
            .bridge
            .request("editor.set_diagnostics", params)
            .map_err(error_from_rpc)?;
        let _ = value;
        Ok(())
    }
}

impl process_run::Host for WasmState {
    fn run(
        &mut self,
        command: String,
        args: Vec<String>,
        timeout_ms: u32,
    ) -> Result<ProcessOutput, Error> {
        self.process
            .run(&command, &args, timeout_ms)
            .map_err(error_from_rpc)
    }
}

fn serialize_params<T: serde::Serialize>(params: T) -> Result<serde_json::Value, Error> {
    serde_json::to_value(params)
        .map_err(|error| error_from_rpc(RpcError::new(PLUGIN_UNAVAILABLE, error.to_string())))
}

fn drain(pipe: &MemoryOutputPipe, offset: usize, stream: &str, bridge: &HostBridge) -> usize {
    let contents = pipe.contents();
    if contents.len() <= offset {
        return offset;
    }
    let text = String::from_utf8_lossy(&contents[offset..]);
    for line in text.lines() {
        bridge.log(format!("{stream}: {line}"));
    }
    contents.len()
}
