use wasmtime::component::{HasData, Linker, Resource};
use wasmtime_wasi::p2::bindings::sockets::network::{ErrorCode, Network};
use wasmtime_wasi::p2::bindings::sync::sockets::network::{Host as NetworkHost, HostNetwork};
use wasmtime_wasi::p2::bindings::sync::sockets::tcp;
use wasmtime_wasi::p2::SocketError;
use wasmtime_wasi::sockets::WasiSocketsCtxView;

use super::bridge::CallDeadline;
use super::net::NetworkPolicy;
use super::state::WasmState;

mod tcp_host;

pub(crate) struct TrackedSockets;

impl HasData for TrackedSockets {
    type Data<'a> = TrackedSocketsView<'a>;
}

pub(crate) struct TrackedSocketsView<'a> {
    inner: WasiSocketsCtxView<'a>,
    policy: NetworkPolicy,
    deadline: CallDeadline,
}

impl<'a> TrackedSocketsView<'a> {
    pub(crate) fn new(
        inner: WasiSocketsCtxView<'a>,
        policy: NetworkPolicy,
        deadline: CallDeadline,
    ) -> Self {
        Self {
            inner,
            policy,
            deadline,
        }
    }

    fn release(&self, handle: u32) {
        self.policy.release_connection(handle);
    }
}

impl tcp::Host for TrackedSocketsView<'_> {}

impl NetworkHost for TrackedSocketsView<'_> {
    fn convert_error_code(&mut self, error: SocketError) -> wasmtime::Result<ErrorCode> {
        <WasiSocketsCtxView<'_> as NetworkHost>::convert_error_code(&mut self.inner, error)
    }

    fn network_error_code(
        &mut self,
        error: Resource<wasmtime_wasi::p2::bindings::sync::sockets::network::Error>,
    ) -> wasmtime::Result<Option<ErrorCode>> {
        <WasiSocketsCtxView<'_> as NetworkHost>::network_error_code(&mut self.inner, error)
    }
}

impl HostNetwork for TrackedSocketsView<'_> {
    fn drop(&mut self, resource: Resource<Network>) -> wasmtime::Result<()> {
        <WasiSocketsCtxView<'_> as HostNetwork>::drop(&mut self.inner, resource)
    }
}

pub(crate) fn add_to_linker(linker: &mut Linker<WasmState>) -> wasmtime::Result<()> {
    linker.allow_shadowing(true);
    tcp::add_to_linker::<WasmState, TrackedSockets>(linker, WasmState::tracked_sockets)
}
