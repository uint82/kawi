use wasmtime::component::{HasData, Linker, Resource};
use wasmtime_wasi::p2::bindings::sockets::network::{ErrorCode, Network};
use wasmtime_wasi::p2::bindings::sync::sockets::network::{Host as NetworkHost, HostNetwork};
use wasmtime_wasi::p2::bindings::sync::sockets::tcp::{
    self, Duration, HostTcpSocket, InputStream, IpAddressFamily, IpSocketAddress, OutputStream,
    Pollable, ShutdownType, TcpSocket,
};
use wasmtime_wasi::p2::SocketError;
use wasmtime_wasi::sockets::WasiSocketsCtxView;

use super::net::NetworkPolicy;
use super::state::WasmState;

pub(crate) struct TrackedSockets;

impl HasData for TrackedSockets {
    type Data<'a> = TrackedSocketsView<'a>;
}

pub(crate) struct TrackedSocketsView<'a> {
    inner: WasiSocketsCtxView<'a>,
    policy: NetworkPolicy,
}

impl<'a> TrackedSocketsView<'a> {
    pub(crate) fn new(inner: WasiSocketsCtxView<'a>, policy: NetworkPolicy) -> Self {
        Self { inner, policy }
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

impl HostTcpSocket for TrackedSocketsView<'_> {
    fn start_bind(
        &mut self,
        this: Resource<TcpSocket>,
        network: Resource<Network>,
        local_address: IpSocketAddress,
    ) -> Result<(), SocketError> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::start_bind(
            &mut self.inner,
            this,
            network,
            local_address,
        )
    }

    fn finish_bind(&mut self, this: Resource<TcpSocket>) -> Result<(), SocketError> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::finish_bind(&mut self.inner, this)
    }

    fn start_connect(
        &mut self,
        this: Resource<TcpSocket>,
        network: Resource<Network>,
        remote_address: IpSocketAddress,
    ) -> Result<(), SocketError> {
        let handle = this.rep();
        let already_tracked = self.policy.is_tracked(handle);
        if !already_tracked && !self.policy.claim_connection(handle) {
            return Err(ErrorCode::AccessDenied.into());
        }
        let result = <WasiSocketsCtxView<'_> as HostTcpSocket>::start_connect(
            &mut self.inner,
            this,
            network,
            remote_address,
        );
        if result.is_err() && !already_tracked {
            self.release(handle);
        }
        result
    }

    fn finish_connect(
        &mut self,
        this: Resource<TcpSocket>,
    ) -> Result<(Resource<InputStream>, Resource<OutputStream>), SocketError> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::finish_connect(&mut self.inner, this)
    }

    fn start_listen(&mut self, this: Resource<TcpSocket>) -> Result<(), SocketError> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::start_listen(&mut self.inner, this)
    }

    fn finish_listen(&mut self, this: Resource<TcpSocket>) -> Result<(), SocketError> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::finish_listen(&mut self.inner, this)
    }

    fn accept(
        &mut self,
        this: Resource<TcpSocket>,
    ) -> Result<
        (
            Resource<TcpSocket>,
            Resource<InputStream>,
            Resource<OutputStream>,
        ),
        SocketError,
    > {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::accept(&mut self.inner, this)
    }

    fn local_address(&mut self, this: Resource<TcpSocket>) -> Result<IpSocketAddress, SocketError> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::local_address(&mut self.inner, this)
    }

    fn remote_address(
        &mut self,
        this: Resource<TcpSocket>,
    ) -> Result<IpSocketAddress, SocketError> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::remote_address(&mut self.inner, this)
    }

    fn is_listening(&mut self, this: Resource<TcpSocket>) -> wasmtime::Result<bool> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::is_listening(&mut self.inner, this)
    }

    fn address_family(&mut self, this: Resource<TcpSocket>) -> wasmtime::Result<IpAddressFamily> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::address_family(&mut self.inner, this)
    }

    fn set_listen_backlog_size(
        &mut self,
        this: Resource<TcpSocket>,
        value: u64,
    ) -> Result<(), SocketError> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::set_listen_backlog_size(
            &mut self.inner,
            this,
            value,
        )
    }

    fn keep_alive_enabled(&mut self, this: Resource<TcpSocket>) -> Result<bool, SocketError> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::keep_alive_enabled(&mut self.inner, this)
    }

    fn set_keep_alive_enabled(
        &mut self,
        this: Resource<TcpSocket>,
        value: bool,
    ) -> Result<(), SocketError> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::set_keep_alive_enabled(
            &mut self.inner,
            this,
            value,
        )
    }

    fn keep_alive_idle_time(&mut self, this: Resource<TcpSocket>) -> Result<Duration, SocketError> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::keep_alive_idle_time(&mut self.inner, this)
    }

    fn set_keep_alive_idle_time(
        &mut self,
        this: Resource<TcpSocket>,
        value: Duration,
    ) -> Result<(), SocketError> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::set_keep_alive_idle_time(
            &mut self.inner,
            this,
            value,
        )
    }

    fn keep_alive_interval(&mut self, this: Resource<TcpSocket>) -> Result<Duration, SocketError> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::keep_alive_interval(&mut self.inner, this)
    }

    fn set_keep_alive_interval(
        &mut self,
        this: Resource<TcpSocket>,
        value: Duration,
    ) -> Result<(), SocketError> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::set_keep_alive_interval(
            &mut self.inner,
            this,
            value,
        )
    }

    fn keep_alive_count(&mut self, this: Resource<TcpSocket>) -> Result<u32, SocketError> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::keep_alive_count(&mut self.inner, this)
    }

    fn set_keep_alive_count(
        &mut self,
        this: Resource<TcpSocket>,
        value: u32,
    ) -> Result<(), SocketError> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::set_keep_alive_count(
            &mut self.inner,
            this,
            value,
        )
    }

    fn hop_limit(&mut self, this: Resource<TcpSocket>) -> Result<u8, SocketError> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::hop_limit(&mut self.inner, this)
    }

    fn set_hop_limit(&mut self, this: Resource<TcpSocket>, value: u8) -> Result<(), SocketError> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::set_hop_limit(&mut self.inner, this, value)
    }

    fn receive_buffer_size(&mut self, this: Resource<TcpSocket>) -> Result<u64, SocketError> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::receive_buffer_size(&mut self.inner, this)
    }

    fn set_receive_buffer_size(
        &mut self,
        this: Resource<TcpSocket>,
        value: u64,
    ) -> Result<(), SocketError> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::set_receive_buffer_size(
            &mut self.inner,
            this,
            value,
        )
    }

    fn send_buffer_size(&mut self, this: Resource<TcpSocket>) -> Result<u64, SocketError> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::send_buffer_size(&mut self.inner, this)
    }

    fn set_send_buffer_size(
        &mut self,
        this: Resource<TcpSocket>,
        value: u64,
    ) -> Result<(), SocketError> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::set_send_buffer_size(
            &mut self.inner,
            this,
            value,
        )
    }

    fn subscribe(&mut self, this: Resource<TcpSocket>) -> wasmtime::Result<Resource<Pollable>> {
        <WasiSocketsCtxView<'_> as HostTcpSocket>::subscribe(&mut self.inner, this)
    }

    fn shutdown(
        &mut self,
        this: Resource<TcpSocket>,
        shutdown_type: ShutdownType,
    ) -> Result<(), SocketError> {
        let handle = this.rep();
        let result = <WasiSocketsCtxView<'_> as HostTcpSocket>::shutdown(
            &mut self.inner,
            this,
            shutdown_type,
        );
        if result.is_ok() && matches!(shutdown_type, ShutdownType::Both) {
            self.release(handle);
        }
        result
    }

    fn drop(&mut self, this: Resource<TcpSocket>) -> wasmtime::Result<()> {
        let handle = this.rep();
        let result = <WasiSocketsCtxView<'_> as HostTcpSocket>::drop(&mut self.inner, this);
        self.release(handle);
        result
    }
}

pub(crate) fn add_to_linker(linker: &mut Linker<WasmState>) -> wasmtime::Result<()> {
    linker.allow_shadowing(true);
    tcp::add_to_linker::<WasmState, TrackedSockets>(linker, WasmState::tracked_sockets)
}
