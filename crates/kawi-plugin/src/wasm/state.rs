use wasmtime::component::ResourceTable;
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;
use wasmtime_wasi::sockets::WasiSocketsView;
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

use crate::protocol::{
    ApplyEditParams, ApplyEditResult, GetStateParams, RpcError, SetDiagnosticsParams,
    UiMessageParams, PLUGIN_UNAVAILABLE,
};

pub use super::bridge::{fail_pending, output_pipes, pending_calls, HostBridge, PendingCalls};

use super::bindings::kawi::plugin::types::{Edit, Error, ProcessOutput, State};
use super::bindings::kawi::plugin::{
    diagnostics_publish, editor_read, editor_write, process_run, types, ui_message,
};
use super::convert::{diagnostics_to_protocol, edits_to_protocol, error_from_rpc, state_from_json};
use super::limits::WasmLimits;
use super::net::NetworkPolicy;
use super::process::ProcessRunner;

pub struct WasmState {
    pub wasi: WasiCtx,
    pub table: ResourceTable,
    pub bridge: HostBridge,
    pub process: ProcessRunner,
    pub network: NetworkPolicy,
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
        let sockets = <Self as WasiSocketsView>::sockets(self);
        super::connection_cap::TrackedSocketsView::new(sockets, policy, deadline)
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
        let params = serde_json::to_value(GetStateParams { include_text }).map_err(|error| {
            error_from_rpc(RpcError::new(PLUGIN_UNAVAILABLE, error.to_string()))
        })?;
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
        let params = serde_json::to_value(ApplyEditParams {
            buffer,
            revision,
            edits,
        })
        .map_err(|error| error_from_rpc(RpcError::new(PLUGIN_UNAVAILABLE, error.to_string())))?;
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

impl ui_message::Host for WasmState {
    fn message(&mut self, text: String) -> Result<(), Error> {
        let params = serde_json::to_value(UiMessageParams { message: text }).map_err(|error| {
            error_from_rpc(RpcError::new(PLUGIN_UNAVAILABLE, error.to_string()))
        })?;
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
        let params = serde_json::to_value(SetDiagnosticsParams {
            buffer,
            revision,
            diagnostics: diagnostics_to_protocol(items),
        })
        .map_err(|error| error_from_rpc(RpcError::new(PLUGIN_UNAVAILABLE, error.to_string())))?;
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
