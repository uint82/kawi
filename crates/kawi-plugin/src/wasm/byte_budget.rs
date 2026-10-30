use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use wasmtime_wasi::p2::{Pollable, StreamError, StreamResult};

use super::bridge::CallDeadline;

pub(crate) const MAX_BYTES_PER_CONNECTION: u64 = 8 * 1024 * 1024;
pub(crate) const MAX_BYTES_PER_SESSION: u64 = 64 * 1024 * 1024;

#[derive(Clone)]
pub(crate) struct SessionBudget {
    remaining: Arc<AtomicU64>,
}

impl SessionBudget {
    pub(crate) fn with_limit(limit: u64) -> Self {
        Self {
            remaining: Arc::new(AtomicU64::new(limit)),
        }
    }

    pub(crate) fn remaining(&self) -> u64 {
        self.remaining.load(Ordering::Relaxed)
    }

    pub(crate) fn try_consume(&self, len: u64) -> StreamResult<()> {
        self.remaining
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |remaining| {
                remaining.checked_sub(len)
            })
            .map(|_| ())
            .map_err(|_| cap_exceeded())
    }
}

#[derive(Clone)]
pub(crate) struct NetworkBudget {
    session: SessionBudget,
}

impl NetworkBudget {
    pub(crate) fn new() -> Self {
        Self {
            session: SessionBudget::with_limit(MAX_BYTES_PER_SESSION),
        }
    }

    pub(crate) fn session(&self) -> SessionBudget {
        self.session.clone()
    }
}

pub(crate) struct ConnectionBudget {
    remaining: u64,
}

impl ConnectionBudget {
    pub(crate) fn new() -> Self {
        Self {
            remaining: MAX_BYTES_PER_CONNECTION,
        }
    }

    pub(crate) fn remaining(&self) -> u64 {
        self.remaining
    }

    pub(crate) fn take(&mut self, len: usize) -> StreamResult<()> {
        let len = u64::try_from(len).map_err(|_| cap_exceeded())?;
        if len > self.remaining {
            return Err(cap_exceeded());
        }
        self.remaining -= len;
        Ok(())
    }
}

pub(crate) async fn wait_ready<P>(pollable: &mut P, deadline: &CallDeadline)
where
    P: Pollable + ?Sized,
{
    let Some(remaining) = deadline.remaining() else {
        pollable.ready().await;
        return;
    };
    let _ = tokio::time::timeout(remaining, pollable.ready()).await;
}

pub(crate) fn ensure_call_active(deadline: &CallDeadline) -> StreamResult<()> {
    if deadline.expired() {
        return Err(StreamError::LastOperationFailed(wasmtime::format_err!(
            "network stream exceeded the invocation budget"
        )));
    }
    Ok(())
}

pub(crate) fn cap_exceeded() -> StreamError {
    StreamError::Closed
}

#[cfg(test)]
#[path = "byte_budget/tests.rs"]
mod tests;
