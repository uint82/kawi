use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};

use kawi_buffer::Buffer;

use super::skipped_buffers_warning;
use crate::app::App;

pub(crate) struct PendingBufferLoad {
    pub(crate) number: usize,
    pub(crate) path: String,
}

struct LoadedBuffer {
    number: usize,
    buffer: Buffer,
}

struct SessionRestorePoll {
    buffers: Vec<LoadedBuffer>,
    finished: bool,
    warning: Option<String>,
}

pub(crate) struct PendingSessionRestore {
    receiver: Receiver<Result<LoadedBuffer, ()>>,
    worker: JoinHandle<()>,
    remaining: usize,
    skipped: usize,
}

impl PendingSessionRestore {
    pub(crate) fn start(loads: Vec<PendingBufferLoad>, wake: Sender<()>) -> Result<Self, String> {
        let remaining = loads.len();
        let (sender, receiver) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("kawi-session-load".to_string())
            .spawn(move || load_buffers(loads, sender, wake))
            .map_err(|error| format!("session: cannot start buffer loader: {error}"))?;
        Ok(Self {
            receiver,
            worker,
            remaining,
            skipped: 0,
        })
    }

    fn poll(&mut self) -> SessionRestorePoll {
        let mut buffers = Vec::new();
        let mut disconnected = false;
        loop {
            match self.receiver.try_recv() {
                Ok(Ok(buffer)) => {
                    buffers.push(buffer);
                    self.remaining = self.remaining.saturating_sub(1);
                }
                Ok(Err(())) => {
                    self.skipped = self.skipped.saturating_add(1);
                    self.remaining = self.remaining.saturating_sub(1);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }
        let finished = self.remaining == 0 || disconnected;
        let warning = if finished {
            restore_warning(self.skipped, disconnected && self.remaining > 0)
        } else {
            None
        };
        SessionRestorePoll {
            buffers,
            finished,
            warning,
        }
    }

    fn join(self) -> Result<(), String> {
        self.worker
            .join()
            .map_err(|_| "session: buffer loader panicked".to_string())
    }
}

impl App {
    pub(crate) fn start_pending_session_restore(&mut self) -> bool {
        if self.pending_session_buffers.is_empty() {
            return false;
        }
        let loads = std::mem::take(&mut self.pending_session_buffers);
        match PendingSessionRestore::start(loads, self.events.wake()) {
            Ok(restore) => self.session_restore = Some(restore),
            Err(error) => self.show_startup_warning(error),
        }
        true
    }

    pub(crate) fn poll_session_restore(&mut self) -> bool {
        let Some(restore) = self.session_restore.as_mut() else {
            return false;
        };
        let poll = restore.poll();
        let mut changed = false;
        for loaded in poll.buffers {
            changed |= self
                .editor
                .replace_parked_buffer(loaded.number, loaded.buffer);
        }
        if let Some(warning) = poll.warning {
            self.show_message(warning);
            changed = true;
        }
        if poll.finished {
            if let Some(restore) = self.session_restore.take() {
                if let Err(error) = restore.join() {
                    self.show_message(error);
                }
            }
            changed = true;
        }
        changed
    }
}

fn load_buffers(
    loads: Vec<PendingBufferLoad>,
    sender: Sender<Result<LoadedBuffer, ()>>,
    wake: Sender<()>,
) {
    for load in loads {
        if sender.send(load_buffer(load)).is_err() {
            return;
        }
        let _ = wake.send(());
    }
}

fn load_buffer(load: PendingBufferLoad) -> Result<LoadedBuffer, ()> {
    let buffer = Buffer::open(&load.path).map_err(|_| ())?;
    Ok(LoadedBuffer {
        number: load.number,
        buffer,
    })
}

fn restore_warning(skipped: usize, interrupted: bool) -> Option<String> {
    let mut warnings = Vec::new();
    if skipped > 0 {
        warnings.push(skipped_buffers_warning(skipped));
    }
    if interrupted {
        warnings.push("session: buffer loader stopped early".to_string());
    }
    (!warnings.is_empty()).then(|| warnings.join("\n"))
}

#[cfg(test)]
mod tests;
