use bytes::Bytes;
use wasmtime::component::{Resource, ResourceTable};
use wasmtime_wasi::async_trait;
use wasmtime_wasi::p2::bindings::sync::sockets::tcp::{OutputStream, TcpSocket};
use wasmtime_wasi::p2::{
    DynOutputStream, OutputStream as OutputStreamTrait, Pollable, SocketError, StreamError,
    StreamResult,
};

use super::bridge::CallDeadline;
use super::byte_budget::{
    cap_exceeded, ensure_call_active, wait_ready, ConnectionBudget, NetworkBudget, SessionBudget,
};

pub(crate) fn cap_output_stream(
    table: &mut ResourceTable,
    socket: u32,
    output: Resource<OutputStream>,
    deadline: CallDeadline,
    budget: NetworkBudget,
) -> Result<Resource<OutputStream>, SocketError> {
    let inner = table.delete(output)?;
    let capped: OutputStream = Box::new(CappedOutputStream::new(inner, deadline, budget.session()));
    let socket: Resource<TcpSocket> = Resource::new_borrow(socket);
    Ok(table.push_child(capped, &socket)?)
}

struct CappedOutputStream {
    inner: DynOutputStream,
    budget: OutboundBudget,
    deadline: CallDeadline,
}

impl CappedOutputStream {
    fn new(inner: DynOutputStream, deadline: CallDeadline, session: SessionBudget) -> Self {
        Self {
            inner,
            budget: OutboundBudget::new(session),
            deadline,
        }
    }
}

#[async_trait]
impl OutputStreamTrait for CappedOutputStream {
    fn write(&mut self, bytes: Bytes) -> StreamResult<()> {
        ensure_call_active(&self.deadline)?;
        self.budget.consume(bytes.len())?;
        self.inner.write(bytes)
    }

    fn flush(&mut self) -> StreamResult<()> {
        self.inner.flush()
    }

    fn check_write(&mut self) -> StreamResult<usize> {
        ensure_call_active(&self.deadline)?;
        let inner = self.inner.check_write()?;
        self.budget.permit(inner)
    }

    async fn cancel(&mut self) {
        self.inner.cancel().await;
    }
}

#[async_trait]
impl Pollable for CappedOutputStream {
    async fn ready(&mut self) {
        wait_ready(self.inner.as_mut(), &self.deadline).await;
    }
}

struct OutboundBudget {
    local: ConnectionBudget,
    session: SessionBudget,
    permit: usize,
    can_drain_check: bool,
}

impl OutboundBudget {
    fn new(session: SessionBudget) -> Self {
        Self {
            local: ConnectionBudget::new(),
            session,
            permit: 0,
            can_drain_check: false,
        }
    }

    fn permit(&mut self, inner: usize) -> StreamResult<usize> {
        if self.can_drain_check {
            self.can_drain_check = false;
            self.permit = inner.max(1);
            return Ok(self.permit);
        }
        let local = self.local.remaining();
        let session = self.session.remaining();
        if local == 0 || session == 0 {
            return Err(cap_exceeded());
        }
        let remaining = usize::try_from(local.min(session)).unwrap_or(usize::MAX);
        self.permit = inner.min(remaining);
        Ok(self.permit)
    }

    fn consume(&mut self, len: usize) -> StreamResult<()> {
        if len > self.permit {
            return Err(StreamError::Trap(wasmtime::format_err!(
                "write exceeds the permitted length"
            )));
        }
        self.local.take(len)?;
        self.permit = 0;
        self.session.try_consume(len as u64)?;
        self.can_drain_check = self.local.remaining() == 0 || self.session.remaining() == 0;
        Ok(())
    }
}

#[cfg(test)]
#[path = "outbound_cap/tests.rs"]
mod tests;
