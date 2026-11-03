use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;

use crate::host::PluginHostEvent;
use crate::protocol::{JobOutputEvent, JobStream, JOB_OUTPUT_EVENT, MAX_JOB_OUTPUT_BYTES};

use super::{lock, send_job_event};

const OUTPUT_BUFFER_BYTES: usize = 8192;

pub(super) struct OutputBudget {
    bytes: AtomicUsize,
    truncated: AtomicBool,
}

impl OutputBudget {
    pub(super) fn new() -> Self {
        Self {
            bytes: AtomicUsize::new(0),
            truncated: AtomicBool::new(false),
        }
    }

    fn allow(&self, size: usize) -> bool {
        let mut current = self.bytes.load(Ordering::Relaxed);
        loop {
            let Some(next) = current
                .checked_add(size)
                .filter(|next| *next <= MAX_JOB_OUTPUT_BYTES)
            else {
                return false;
            };
            match self.bytes.compare_exchange_weak(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(actual) => current = actual,
            }
        }
    }

    fn mark_truncated(&self) -> bool {
        !self.truncated.swap(true, Ordering::Relaxed)
    }
}

pub(super) struct OutputTracker {
    remaining: Mutex<usize>,
    drained: Condvar,
}

impl OutputTracker {
    pub(super) fn new(streams: usize) -> Arc<Self> {
        Arc::new(Self {
            remaining: Mutex::new(streams),
            drained: Condvar::new(),
        })
    }

    fn finish(&self) {
        let mut remaining = lock(&self.remaining);
        *remaining = remaining.saturating_sub(1);
        if *remaining == 0 {
            self.drained.notify_all();
        }
    }

    pub(super) fn wait(&self, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        let mut remaining = lock(&self.remaining);
        while *remaining > 0 {
            let now = Instant::now();
            if now >= deadline {
                return;
            }
            let wait = deadline.saturating_duration_since(now);
            remaining = self
                .drained
                .wait_timeout(remaining, wait)
                .unwrap_or_else(|error| error.into_inner())
                .0;
        }
    }
}

#[derive(Default)]
pub(super) struct Utf8Stream {
    pending: Vec<u8>,
}

impl Utf8Stream {
    pub(super) fn push(&mut self, bytes: &[u8]) -> String {
        self.pending.extend_from_slice(bytes);
        match std::str::from_utf8(&self.pending) {
            Ok(text) => {
                let output = text.to_string();
                self.pending.clear();
                output
            }
            Err(error) if error.error_len().is_none() => {
                let valid = error.valid_up_to();
                let output = String::from_utf8_lossy(&self.pending[..valid]).into_owned();
                self.pending.drain(..valid);
                output
            }
            Err(_) => {
                let output = String::from_utf8_lossy(&self.pending).into_owned();
                self.pending.clear();
                output
            }
        }
    }

    pub(super) fn finish(&mut self) -> String {
        if self.pending.is_empty() {
            return String::new();
        }
        let output = String::from_utf8_lossy(&self.pending).into_owned();
        self.pending.clear();
        output
    }
}

pub(super) struct OutputStream {
    pub(super) plugin: String,
    pub(super) job: u64,
    pub(super) stream: JobStream,
    pub(super) budget: Arc<OutputBudget>,
    pub(super) tracker: Arc<OutputTracker>,
}

pub(super) fn spawn_output(
    mut reader: impl Read + Send + 'static,
    events: &std::sync::mpsc::Sender<PluginHostEvent>,
    wake: &std::sync::mpsc::Sender<()>,
    output: OutputStream,
) {
    let events = events.clone();
    let wake = wake.clone();
    let OutputStream {
        plugin,
        job,
        stream,
        budget,
        tracker,
    } = output;
    thread::spawn(move || {
        output_loop(&mut reader, &events, &wake, &plugin, job, stream, &budget);
        tracker.finish();
    });
}

fn output_loop(
    reader: &mut impl Read,
    events: &std::sync::mpsc::Sender<PluginHostEvent>,
    wake: &std::sync::mpsc::Sender<()>,
    plugin: &str,
    job: u64,
    stream: JobStream,
    budget: &OutputBudget,
) {
    let mut buffer = [0_u8; OUTPUT_BUFFER_BYTES];
    let mut utf8 = Utf8Stream::default();
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => {
                send_output(events, wake, plugin, job, stream, utf8.finish(), false);
                return;
            }
            Ok(read) => {
                if !budget.allow(read) {
                    if budget.mark_truncated() {
                        send_output(events, wake, plugin, job, stream, String::new(), true);
                    }
                    continue;
                }
                send_output(
                    events,
                    wake,
                    plugin,
                    job,
                    stream,
                    utf8.push(&buffer[..read]),
                    false,
                );
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => {
                send_output(events, wake, plugin, job, stream, utf8.finish(), false);
                return;
            }
        }
    }
}

fn send_output(
    events: &std::sync::mpsc::Sender<PluginHostEvent>,
    wake: &std::sync::mpsc::Sender<()>,
    plugin: &str,
    job: u64,
    stream: JobStream,
    data: String,
    truncated: bool,
) {
    if data.is_empty() && !truncated {
        return;
    }
    send_job_event(
        events,
        wake,
        plugin,
        JOB_OUTPUT_EVENT,
        json!(JobOutputEvent {
            job,
            stream,
            data,
            truncated,
        }),
    );
}
