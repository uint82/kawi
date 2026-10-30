use bytes::Bytes;
use wasmtime::component::{Resource, ResourceTable};
use wasmtime_wasi::async_trait;
use wasmtime_wasi::p2::bindings::sync::sockets::tcp::{InputStream, TcpSocket};
use wasmtime_wasi::p2::{
    DynInputStream, InputStream as InputStreamTrait, Pollable, SocketError, StreamResult,
};

use super::bridge::CallDeadline;
use super::byte_budget::{
    cap_exceeded, ensure_call_active, wait_ready, ConnectionBudget, NetworkBudget, SessionBudget,
};

pub(crate) fn cap_input_stream(
    table: &mut ResourceTable,
    socket: u32,
    input: Resource<InputStream>,
    deadline: CallDeadline,
    budget: NetworkBudget,
) -> Result<Resource<InputStream>, SocketError> {
    let inner = table.delete(input)?;
    let capped: InputStream = Box::new(CappedInputStream::new(inner, deadline, budget.session()));
    let socket: Resource<TcpSocket> = Resource::new_borrow(socket);
    Ok(table.push_child(capped, &socket)?)
}

struct CappedInputStream {
    inner: DynInputStream,
    local: ConnectionBudget,
    session: SessionBudget,
    deadline: CallDeadline,
}

impl CappedInputStream {
    fn new(inner: DynInputStream, deadline: CallDeadline, session: SessionBudget) -> Self {
        Self {
            inner,
            local: ConnectionBudget::new(),
            session,
            deadline,
        }
    }

    fn read_limit(&self, size: usize) -> StreamResult<usize> {
        if size == 0 {
            return Ok(0);
        }
        ensure_call_active(&self.deadline)?;
        let local = self.local.remaining();
        let session = self.session.remaining();
        if local == 0 || session == 0 {
            return Err(cap_exceeded());
        }
        let remaining = usize::try_from(local.min(session)).unwrap_or(usize::MAX);
        Ok(size.min(remaining))
    }
}

#[async_trait]
impl InputStreamTrait for CappedInputStream {
    fn read(&mut self, size: usize) -> StreamResult<Bytes> {
        let size = self.read_limit(size)?;
        if size == 0 {
            return Ok(Bytes::new());
        }
        let bytes = self.inner.read(size)?;
        let len = bytes.len();
        self.local.take(len)?;
        self.session.try_consume(len as u64)?;
        Ok(bytes)
    }

    async fn cancel(&mut self) {
        self.inner.cancel().await;
    }
}

#[async_trait]
impl Pollable for CappedInputStream {
    async fn ready(&mut self) {
        wait_ready(self.inner.as_mut(), &self.deadline).await;
    }
}

#[cfg(test)]
#[path = "inbound_cap/tests.rs"]
mod tests;
