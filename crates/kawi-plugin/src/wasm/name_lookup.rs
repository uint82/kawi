use std::net::IpAddr;
use std::vec;

use wasmtime::component::{HasData, Linker, Resource};
use wasmtime::Result;
use wasmtime_wasi::p2::bindings::sync::sockets::network::{
    Error, ErrorCode, Host as NetworkHost, HostNetwork, IpAddress, Network,
};
use wasmtime_wasi::p2::{subscribe, DynPollable, Pollable, SocketError};
use wasmtime_wasi::sockets::{WasiSocketsCtxView, WasiSocketsView};

use super::net::NetworkPolicy;
use super::state::WasmState;

mod bindings {
    wasmtime::component::bindgen!({
        path: "wit/name-lookup",
        world: "name-lookup",
        with: {
            "wasi:io/poll.pollable": wasmtime_wasi::p2::DynPollable,
            "wasi:sockets/network": crate::wasm::name_lookup::network_alias,
            "wasi:sockets/ip-name-lookup.resolve-address-stream": crate::wasm::name_lookup::ResolveAddressStream,
        },
    });
}

use bindings::wasi::sockets::ip_name_lookup::{Host, HostResolveAddressStream};

pub mod network_alias {
    pub use wasmtime_wasi::p2::bindings::sync::sockets::network::*;

    #[allow(dead_code)]
    pub fn add_to_linker<T, D>(
        linker: &mut wasmtime::component::Linker<T>,
        host_getter: fn(&mut T) -> D::Data<'_>,
    ) -> wasmtime::Result<()>
    where
        D: HostWithStore<T>,
        for<'a> D::Data<'a>: Host,
        T: 'static,
    {
        wasmtime_wasi::p2::bindings::sync::sockets::network::add_to_linker::<T, D>(
            linker,
            &Default::default(),
            host_getter,
        )
    }
}

pub struct ResolveAddressStream {
    addresses: vec::IntoIter<IpAddr>,
}

#[wasmtime_wasi::async_trait]
impl Pollable for ResolveAddressStream {
    async fn ready(&mut self) {}
}

pub(crate) struct FilteredNameLookup;

impl HasData for FilteredNameLookup {
    type Data<'a> = FilteredNameLookupView<'a>;
}

pub(crate) struct FilteredNameLookupView<'a> {
    inner: WasiSocketsCtxView<'a>,
    policy: NetworkPolicy,
}

impl NetworkHost for FilteredNameLookupView<'_> {
    fn convert_error_code(&mut self, error: SocketError) -> Result<ErrorCode> {
        <WasiSocketsCtxView<'_> as NetworkHost>::convert_error_code(&mut self.inner, error)
    }

    fn network_error_code(&mut self, error: Resource<Error>) -> Result<Option<ErrorCode>> {
        <WasiSocketsCtxView<'_> as NetworkHost>::network_error_code(&mut self.inner, error)
    }
}

impl HostNetwork for FilteredNameLookupView<'_> {
    fn drop(&mut self, resource: Resource<Network>) -> Result<()> {
        <WasiSocketsCtxView<'_> as HostNetwork>::drop(&mut self.inner, resource)
    }
}

impl Host for FilteredNameLookupView<'_> {
    fn resolve_addresses(
        &mut self,
        network: Resource<Network>,
        name: String,
    ) -> Result<Resource<ResolveAddressStream>, ErrorCode> {
        _ = self
            .inner
            .table
            .get(&network)
            .map_err(|_| ErrorCode::Unknown)?;
        let addresses = self.policy.resolve_name(&name).map_err(|error| {
            if error.kind() == std::io::ErrorKind::PermissionDenied {
                ErrorCode::AccessDenied
            } else {
                ErrorCode::NameUnresolvable
            }
        })?;
        if addresses.is_empty() {
            return Err(ErrorCode::NameUnresolvable);
        }
        let stream = ResolveAddressStream {
            addresses: addresses.into_iter(),
        };
        self.inner
            .table
            .push(stream)
            .map_err(|_| ErrorCode::Unknown)
    }
}

impl HostResolveAddressStream for FilteredNameLookupView<'_> {
    fn resolve_next_address(
        &mut self,
        resource: Resource<ResolveAddressStream>,
    ) -> Result<Option<IpAddress>, ErrorCode> {
        let stream = self
            .inner
            .table
            .get_mut(&resource)
            .map_err(|_| ErrorCode::Unknown)?;
        Ok(stream.addresses.next().map(Into::into))
    }

    fn subscribe(&mut self, resource: Resource<ResolveAddressStream>) -> Resource<DynPollable> {
        subscribe(self.inner.table, resource).expect("subscribe pollable")
    }

    fn drop(&mut self, resource: Resource<ResolveAddressStream>) -> Result<()> {
        self.inner.table.delete(resource)?;
        Ok(())
    }
}

pub(crate) fn add_to_linker(linker: &mut Linker<WasmState>) -> Result<()> {
    linker.allow_shadowing(true);
    bindings::wasi::sockets::ip_name_lookup::add_to_linker::<WasmState, FilteredNameLookup>(
        linker,
        |state| {
            let policy = state.network.clone();
            let inner = state.sockets();
            FilteredNameLookupView { inner, policy }
        },
    )
}
