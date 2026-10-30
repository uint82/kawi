use bytes::Bytes;
use wasmtime::component::{Resource, ResourceTable};
use wasmtime_wasi::async_trait;
use wasmtime_wasi::p2::bindings::sync::sockets::tcp::{OutputStream, TcpSocket};
use wasmtime_wasi::p2::{
    DynOutputStream, OutputStream as OutputStreamTrait, Pollable, SocketError, StreamError,
    StreamResult,
};

use super::bridge::CallDeadline;

const MAX_OUTBOUND_BYTES_PER_CONNECTION: u64 = 8 * 1024 * 1024;

pub(crate) fn cap_output_stream(
    table: &mut ResourceTable,
    socket: u32,
    output: Resource<OutputStream>,
    deadline: CallDeadline,
) -> Result<Resource<OutputStream>, SocketError> {
    let inner = table.delete(output)?;
    let capped: OutputStream = Box::new(CappedOutputStream::new(inner, deadline));
    let socket: Resource<TcpSocket> = Resource::new_borrow(socket);
    Ok(table.push_child(capped, &socket)?)
}

struct CappedOutputStream {
    inner: DynOutputStream,
    budget: OutboundBudget,
    deadline: CallDeadline,
}

impl CappedOutputStream {
    fn new(inner: DynOutputStream, deadline: CallDeadline) -> Self {
        Self {
            inner,
            budget: OutboundBudget::new(MAX_OUTBOUND_BYTES_PER_CONNECTION),
            deadline,
        }
    }

    fn ensure_call_active(&self) -> StreamResult<()> {
        if self.deadline.expired() {
            return Err(StreamError::LastOperationFailed(wasmtime::format_err!(
                "network write exceeded the invocation budget"
            )));
        }
        Ok(())
    }
}

#[async_trait]
impl OutputStreamTrait for CappedOutputStream {
    fn write(&mut self, bytes: Bytes) -> StreamResult<()> {
        self.ensure_call_active()?;
        self.budget.consume(bytes.len())?;
        self.inner.write(bytes)
    }

    fn flush(&mut self) -> StreamResult<()> {
        self.inner.flush()
    }

    fn check_write(&mut self) -> StreamResult<usize> {
        self.ensure_call_active()?;
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
        let Some(remaining) = self.deadline.remaining() else {
            self.inner.ready().await;
            return;
        };
        let _ = tokio::time::timeout(remaining, self.inner.ready()).await;
    }
}

struct OutboundBudget {
    remaining: u64,
    permit: usize,
    can_drain_check: bool,
}

impl OutboundBudget {
    fn new(limit: u64) -> Self {
        Self {
            remaining: limit,
            permit: 0,
            can_drain_check: false,
        }
    }

    fn permit(&mut self, inner: usize) -> StreamResult<usize> {
        if self.remaining > 0 {
            let remaining = usize::try_from(self.remaining).unwrap_or(usize::MAX);
            self.permit = inner.min(remaining);
            return Ok(self.permit);
        }
        if self.can_drain_check {
            self.can_drain_check = false;
            self.permit = inner.max(1);
            return Ok(self.permit);
        }
        Err(cap_exceeded())
    }

    fn consume(&mut self, len: usize) -> StreamResult<()> {
        if len > self.permit {
            return Err(StreamError::Trap(wasmtime::format_err!(
                "write exceeds the permitted length"
            )));
        }
        let len = u64::try_from(len).map_err(|_| cap_exceeded())?;
        if len > self.remaining {
            return Err(cap_exceeded());
        }
        self.remaining -= len;
        self.permit = 0;
        self.can_drain_check = self.remaining == 0;
        Ok(())
    }
}

fn cap_exceeded() -> StreamError {
    StreamError::Closed
}

#[cfg(test)]
#[path = "outbound_cap/tests.rs"]
mod tests;
