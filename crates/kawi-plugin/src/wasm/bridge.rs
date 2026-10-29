use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;

use crate::host::{send_event, PluginHostEvent};
use crate::protocol::{RpcError, PLUGIN_UNAVAILABLE};

const HOST_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const OUTPUT_LIMIT: usize = 64 * 1024;

pub type PendingCalls = Arc<Mutex<BTreeMap<u64, Sender<Result<Value, RpcError>>>>>;

#[derive(Clone)]
pub struct CallDeadline {
    timeout: Duration,
    deadline: Arc<Mutex<Option<Instant>>>,
}

impl CallDeadline {
    fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            deadline: Arc::new(Mutex::new(None)),
        }
    }

    fn begin(&self) {
        *self
            .deadline
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(Instant::now() + self.timeout);
    }

    fn clear(&self) {
        *self
            .deadline
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = None;
    }

    fn remaining(&self) -> Option<Duration> {
        self.deadline
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
    }

    fn expired(&self) -> bool {
        self.remaining()
            .is_some_and(|remaining| remaining.is_zero())
    }
}

#[derive(Clone)]
pub struct HostBridge {
    plugin: String,
    events: Sender<PluginHostEvent>,
    wake: Sender<()>,
    next_id: Arc<AtomicU64>,
    pending: PendingCalls,
    deadline: CallDeadline,
    cancel: Arc<AtomicBool>,
}

impl HostBridge {
    pub fn new(
        plugin: String,
        events: Sender<PluginHostEvent>,
        wake: Sender<()>,
        pending: PendingCalls,
        cancel: Arc<AtomicBool>,
        timeout: Duration,
    ) -> Self {
        Self {
            plugin,
            events,
            wake,
            next_id: Arc::new(AtomicU64::new(1)),
            pending,
            deadline: CallDeadline::new(timeout),
            cancel,
        }
    }

    pub(crate) fn begin_call(&self) {
        self.deadline.begin();
    }

    pub(crate) fn clear_call(&self) {
        self.deadline.clear();
    }

    pub(crate) fn call_expired(&self) -> bool {
        self.deadline.expired()
    }

    pub(crate) fn request(&self, method: &str, params: Value) -> Result<Value, RpcError> {
        let timeout = self
            .deadline
            .remaining()
            .unwrap_or(HOST_REQUEST_TIMEOUT)
            .min(HOST_REQUEST_TIMEOUT);
        if timeout.is_zero() || self.cancel.load(Ordering::SeqCst) {
            return Err(RpcError::new(
                PLUGIN_UNAVAILABLE,
                format!("host request canceled: {method}"),
            ));
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (response_tx, response_rx) = mpsc::channel();
        self.pending
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(id, response_tx);
        if self.cancel.load(Ordering::SeqCst) {
            self.pending
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .remove(&id);
            return Err(RpcError::new(PLUGIN_UNAVAILABLE, "plugin restarting"));
        }
        send_event(
            &self.events,
            &self.wake,
            PluginHostEvent::HostRequest {
                plugin: self.plugin.clone(),
                id: Value::from(id),
                method: method.to_string(),
                params,
            },
        );
        let result = response_rx.recv_timeout(timeout);
        self.pending
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(&id);
        if self.cancel.load(Ordering::SeqCst) {
            return Err(RpcError::new(PLUGIN_UNAVAILABLE, "plugin restarting"));
        }
        result.unwrap_or_else(|_| {
            Err(RpcError::new(
                PLUGIN_UNAVAILABLE,
                format!("host request timed out: {method}"),
            ))
        })
    }

    pub(crate) fn log(&self, line: String) {
        send_event(
            &self.events,
            &self.wake,
            PluginHostEvent::Log {
                plugin: self.plugin.clone(),
                line,
            },
        );
    }
}

pub fn pending_calls() -> PendingCalls {
    Arc::new(Mutex::new(BTreeMap::new()))
}

pub fn output_pipes() -> (MemoryOutputPipe, MemoryOutputPipe) {
    (
        MemoryOutputPipe::new(OUTPUT_LIMIT),
        MemoryOutputPipe::new(OUTPUT_LIMIT),
    )
}

pub fn fail_pending(pending: &PendingCalls, error: RpcError) {
    let mut pending = pending.lock().unwrap_or_else(|error| error.into_inner());
    let calls = std::mem::take(&mut *pending);
    for (_, sender) in calls {
        let _ = sender.send(Err(error.clone()));
    }
}
