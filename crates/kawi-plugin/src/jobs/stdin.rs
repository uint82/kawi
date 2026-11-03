use std::io::Write;
use std::process::ChildStdin;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::protocol::{RpcError, JOB_CLOSED, JOB_QUEUE_FULL};

use super::lock;

const STDIN_QUEUE_CAPACITY: usize = 16;

pub(super) enum StdinCommand {
    Data(Vec<u8>),
    Close,
}

#[derive(Clone)]
pub(super) struct StdinHandle {
    sender: Arc<Mutex<Option<SyncSender<StdinCommand>>>>,
}

impl StdinHandle {
    pub(super) fn piped(sender: SyncSender<StdinCommand>) -> Self {
        Self {
            sender: Arc::new(Mutex::new(Some(sender))),
        }
    }

    pub(super) fn disabled() -> Self {
        Self {
            sender: Arc::new(Mutex::new(None)),
        }
    }

    pub(super) fn write(&self, data: Vec<u8>) -> Result<(), RpcError> {
        let sender = lock(&self.sender);
        let Some(sender) = sender.as_ref() else {
            return Err(RpcError::new(JOB_CLOSED, "job stdin is closed"));
        };
        sender
            .try_send(StdinCommand::Data(data))
            .map_err(|_| RpcError::new(JOB_QUEUE_FULL, "job input queue is full"))
    }

    pub(super) fn close(&self) -> Result<(), RpcError> {
        let mut sender = lock(&self.sender);
        let Some(sender) = sender.take() else {
            return Ok(());
        };
        let _ = sender.try_send(StdinCommand::Close);
        Ok(())
    }

    pub(super) fn close_after_exit(&self) {
        lock(&self.sender).take();
    }
}

pub(super) fn spawn_stdin(stdin: ChildStdin) -> StdinHandle {
    let (sender, receiver) = mpsc::sync_channel(STDIN_QUEUE_CAPACITY);
    thread::spawn(move || stdin_loop(stdin, receiver));
    StdinHandle::piped(sender)
}

fn stdin_loop(mut stdin: ChildStdin, receiver: Receiver<StdinCommand>) {
    for command in receiver {
        match command {
            StdinCommand::Data(data) => {
                if stdin.write_all(&data).and_then(|_| stdin.flush()).is_err() {
                    return;
                }
            }
            StdinCommand::Close => return,
        }
    }
}
