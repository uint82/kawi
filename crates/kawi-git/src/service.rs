use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use crate::Status;

pub struct StatusEvent {
    pub generation: u64,
    pub result: Result<Option<Status>>,
}

pub struct StatusService {
    requests: Option<Sender<Request>>,
    events: Receiver<StatusEvent>,
    worker: Option<JoinHandle<()>>,
}

struct Request {
    generation: u64,
    path: PathBuf,
    wake: Sender<()>,
}

impl StatusService {
    pub fn new() -> Result<Self> {
        let (requests, request_receiver) = mpsc::channel();
        let (event_sender, events) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("kawi-git-status".to_string())
            .spawn(move || run(request_receiver, event_sender))
            .context("spawn git status worker")?;
        Ok(Self {
            requests: Some(requests),
            events,
            worker: Some(worker),
        })
    }

    pub fn request(&self, generation: u64, path: PathBuf, wake: Sender<()>) -> Result<()> {
        self.requests
            .as_ref()
            .context("git status service closed")?
            .send(Request {
                generation,
                path,
                wake,
            })
            .context("send git status request")
    }

    pub fn try_recv(&self) -> Option<StatusEvent> {
        self.events.try_recv().ok()
    }

    pub fn shutdown(&mut self) {
        self.requests.take();
        let Some(worker) = self.worker.take() else {
            return;
        };
        if worker.is_finished() {
            let _ = worker.join();
        }
    }
}

impl Drop for StatusService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn run(requests: Receiver<Request>, events: Sender<StatusEvent>) {
    while let Ok(mut request) = requests.recv() {
        while let Ok(next) = requests.try_recv() {
            request = next;
        }
        let result = Status::load(&request.path);
        if events
            .send(StatusEvent {
                generation: request.generation,
                result,
            })
            .is_err()
        {
            return;
        }
        let _ = request.wake.send(());
    }
}
