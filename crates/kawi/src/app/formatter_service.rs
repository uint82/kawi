use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use kawi_lsp::FormatterConfig;

use super::formatter::{self, FormatError};

pub(super) struct FormatResult {
    pub(super) request: u64,
    pub(super) result: Result<String, FormatError>,
}

pub(super) struct FormatterService {
    requests: Option<Sender<Request>>,
    results: Receiver<FormatResult>,
    worker: Option<JoinHandle<()>>,
    next_request: u64,
}

struct Request {
    request: u64,
    path: PathBuf,
    formatter: FormatterConfig,
    text: String,
}

impl FormatterService {
    pub(super) fn new(wake: Sender<()>) -> Result<Self, String> {
        let (requests, request_receiver) = mpsc::channel();
        let (result_sender, results) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("kawi-formatter".to_string())
            .spawn(move || run(request_receiver, result_sender, wake))
            .map_err(|error| format!("cannot start formatter: {error}"))?;
        Ok(Self {
            requests: Some(requests),
            results,
            worker: Some(worker),
            next_request: 0,
        })
    }

    pub(super) fn request(
        &mut self,
        path: PathBuf,
        formatter: FormatterConfig,
        text: String,
    ) -> Result<u64, String> {
        self.next_request = self.next_request.saturating_add(1);
        let request = self.next_request;
        self.requests
            .as_ref()
            .ok_or_else(|| "formatter service closed".to_string())?
            .send(Request {
                request,
                path,
                formatter,
                text,
            })
            .map_err(|_| "formatter service stopped".to_string())?;
        Ok(request)
    }

    pub(super) fn try_recv(&self) -> Option<FormatResult> {
        self.results.try_recv().ok()
    }

    pub(super) fn shutdown(&mut self) {
        self.requests.take();
        let Some(worker) = self.worker.take() else {
            return;
        };
        if worker.is_finished() {
            let _ = worker.join();
        }
    }
}

impl Drop for FormatterService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn run(requests: Receiver<Request>, results: Sender<FormatResult>, wake: Sender<()>) {
    while let Ok(request) = requests.recv() {
        let result = formatter::format(&request.formatter, &request.path, &request.text);
        if results
            .send(FormatResult {
                request: request.request,
                result,
            })
            .is_err()
        {
            return;
        }
        let _ = wake.send(());
    }
}
