use crate::highlight::{Document, Runtime, RuntimeCache};
use crate::language::LanguageId;
use crate::limits::MAX_FILE_BYTES;
use crate::span::{SyntaxKey, SyntaxRequest, SyntaxResult, SyntaxSpan};
use std::ops::Range;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct SyntaxService {
    shared: Arc<Shared>,
    results: Receiver<SyntaxResult>,
    worker: Option<JoinHandle<()>>,
}

struct Shared {
    request: Mutex<Option<Queued>>,
    ready: Condvar,
    shutdown: AtomicBool,
    generation: AtomicU64,
    runtimes: RuntimeCache,
}

struct Queued {
    generation: u64,
    request: SyntaxRequest,
}

struct Worker {
    runtimes: RuntimeCache,
    document: Option<Document>,
    document_id: Option<u64>,
    language: Option<LanguageId>,
}

impl SyntaxService {
    pub fn new() -> Self {
        let (wake, _) = mpsc::channel();
        Self::with_wake(wake)
    }

    pub fn with_wake(wake: Sender<()>) -> Self {
        let (sender, results) = mpsc::channel();
        let shared = Arc::new(Shared {
            request: Mutex::new(None),
            ready: Condvar::new(),
            shutdown: AtomicBool::new(false),
            generation: AtomicU64::new(0),
            runtimes: RuntimeCache::new(),
        });
        let worker = {
            let shared = Arc::clone(&shared);
            thread::Builder::new()
                .name("kawi-syntax".to_string())
                .spawn(move || run(shared, sender, wake))
                .ok()
        };
        Self {
            shared,
            results,
            worker,
        }
    }

    pub fn submit(&self, request: SyntaxRequest) -> u64 {
        let generation = self
            .shared
            .generation
            .fetch_add(1, Ordering::SeqCst)
            .saturating_add(1);
        let mut slot = self
            .shared
            .request
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(queued) = slot.as_mut() {
            if can_merge(queued, &request) {
                merge_request(queued, generation, request);
                self.shared.ready.notify_one();
                return generation;
            }
        }
        *slot = Some(Queued {
            generation,
            request,
        });
        self.shared.ready.notify_one();
        generation
    }

    /// Paints the visible window before the first frame, sharing compiled
    /// runtimes with the worker.
    pub fn highlight_now(&self, request: SyntaxRequest) -> SyntaxResult {
        let generation = self
            .shared
            .generation
            .fetch_add(1, Ordering::SeqCst)
            .saturating_add(1);
        let key = request.key;
        let (window, spans) = if request.text.len_bytes() > MAX_FILE_BYTES {
            (key.top..key.end, Vec::new())
        } else {
            let Some(runtime) = self.shared.runtimes.get(key.language) else {
                return Self::empty_result(key, generation);
            };
            let Some(parser) = runtime.source().parser() else {
                return Self::empty_result(key, generation);
            };
            let mut document =
                Document::with_parser(key.language, parser, self.shared.runtimes.clone());
            document.first_paint(&request, &runtime)
        };
        SyntaxResult {
            key,
            generation,
            window,
            spans,
        }
    }

    pub fn try_recv(&self) -> Option<SyntaxResult> {
        self.results.try_recv().ok()
    }

    fn empty_result(key: SyntaxKey, generation: u64) -> SyntaxResult {
        SyntaxResult {
            key,
            generation,
            window: key.top..key.end,
            spans: Vec::new(),
        }
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Option<SyntaxResult> {
        self.results.recv_timeout(timeout).ok()
    }

    pub fn detach(&mut self) {
        self.signal_shutdown();
        self.worker.take();
    }

    fn signal_shutdown(&self) {
        {
            let _slot = self
                .shared
                .request
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            self.shared.shutdown.store(true, Ordering::SeqCst);
        }
        self.shared.ready.notify_one();
    }
}

impl Default for SyntaxService {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SyntaxService {
    fn drop(&mut self) {
        self.signal_shutdown();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run(shared: Arc<Shared>, sender: Sender<SyntaxResult>, wake: Sender<()>) {
    let mut worker = Worker::new(shared.runtimes.clone());
    while let Some(queued) = next_request(&shared) {
        let result = worker.highlight(queued);
        if sender.send(result).is_err() {
            return;
        }
        let _ = wake.send(());
    }
}

fn can_merge(queued: &Queued, request: &SyntaxRequest) -> bool {
    let current = &queued.request;
    current.key.document == request.key.document
        && current.key.language == request.key.language
        && !current.edits.is_empty()
        && current.key.revision == request.base_revision
}

fn merge_request(queued: &mut Queued, generation: u64, request: SyntaxRequest) {
    let current = &mut queued.request;
    current.edits.extend(request.edits);
    current.key = request.key;
    current.text = request.text;
    queued.generation = generation;
}

fn next_request(shared: &Shared) -> Option<Queued> {
    let mut slot = shared
        .request
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    loop {
        if shared.shutdown.load(Ordering::SeqCst) {
            return None;
        }
        if let Some(request) = slot.take() {
            return Some(request);
        }
        slot = shared
            .ready
            .wait(slot)
            .unwrap_or_else(|error| error.into_inner());
    }
}

impl Worker {
    fn new(runtimes: RuntimeCache) -> Self {
        Self {
            runtimes,
            document: None,
            document_id: None,
            language: None,
        }
    }

    fn highlight(&mut self, queued: Queued) -> SyntaxResult {
        let key = queued.request.key;
        let (window, spans) = if queued.request.text.len_bytes() > MAX_FILE_BYTES {
            (key.top..key.end, Vec::new())
        } else {
            self.spans(key, &queued.request)
        };
        SyntaxResult {
            key,
            generation: queued.generation,
            window,
            spans,
        }
    }

    fn spans(
        &mut self,
        key: SyntaxKey,
        request: &SyntaxRequest,
    ) -> (Range<usize>, Vec<SyntaxSpan>) {
        let Some(runtime) = self.runtimes.get(key.language) else {
            return (key.top..key.end, Vec::new());
        };
        self.prepare(key, &runtime);
        let Some(document) = self.document.as_mut() else {
            return (key.top..key.end, Vec::new());
        };
        document.update(request, &runtime)
    }

    fn prepare(&mut self, key: SyntaxKey, runtime: &Runtime) {
        if self.document_id == Some(key.document) && self.language == Some(key.language) {
            return;
        }
        self.document = runtime
            .source()
            .parser()
            .map(|parser| Document::with_parser(key.language, parser, self.runtimes.clone()));
        self.document_id = Some(key.document);
        self.language = Some(key.language);
    }
}

#[cfg(test)]
mod tests;
