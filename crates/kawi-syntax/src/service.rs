use crate::highlight::{Document, Runtime};
use crate::language::LanguageId;
use crate::limits::MAX_FILE_BYTES;
use crate::span::{SyntaxKey, SyntaxRequest, SyntaxResult, SyntaxSpan};
use std::collections::HashMap;
use std::ops::Range;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

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
}

struct Queued {
    generation: u64,
    request: SyntaxRequest,
}

struct Worker {
    runtimes: HashMap<LanguageId, Option<Runtime>>,
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
        *slot = Some(Queued {
            generation,
            request,
        });
        self.shared.ready.notify_one();
        generation
    }

    pub fn try_recv(&self) -> Option<SyntaxResult> {
        self.results.try_recv().ok()
    }
}

impl Default for SyntaxService {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SyntaxService {
    fn drop(&mut self) {
        self.shared.shutdown.store(true, Ordering::SeqCst);
        self.shared.ready.notify_one();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run(shared: Arc<Shared>, sender: Sender<SyntaxResult>, wake: Sender<()>) {
    let mut worker = Worker::new();
    while let Some(queued) = next_request(&shared) {
        let result = worker.highlight(queued);
        if sender.send(result).is_err() {
            return;
        }
        let _ = wake.send(());
    }
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
    fn new() -> Self {
        Self {
            runtimes: HashMap::new(),
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
        self.prepare(key);
        let runtime = self
            .runtimes
            .entry(key.language)
            .or_insert_with(|| Runtime::new(key.language));
        let Some(runtime) = runtime.as_ref() else {
            return (key.top..key.end, Vec::new());
        };
        let Some(document) = self.document.as_mut() else {
            return (key.top..key.end, Vec::new());
        };
        document.update(request, runtime)
    }

    fn prepare(&mut self, key: SyntaxKey) {
        if self.document_id == Some(key.document) && self.language == Some(key.language) {
            return;
        }
        self.document = Some(Document::new(key.language));
        self.document_id = Some(key.document);
        self.language = Some(key.language);
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use ropey::Rope;

    use super::*;
    use crate::registry::registry;

    fn request(text: &str, rows: usize) -> SyntaxRequest {
        SyntaxRequest {
            key: SyntaxKey {
                document: 1,
                revision: 1,
                top: 0,
                end: rows,
                language: LanguageId::Rust,
            },
            text: Rope::from_str(text),
        }
    }

    #[test]
    fn service_notifies_the_wake_channel() {
        let (wake, wakes) = mpsc::channel();
        let service = SyntaxService::with_wake(wake);
        service.submit(request("fn main() {}\n", 1));
        wakes
            .recv_timeout(Duration::from_secs(5))
            .expect("wake notification");
        assert!(service.try_recv().is_some());
    }

    #[test]
    fn service_returns_rust_spans() {
        let service = SyntaxService::new();
        service.submit(request("// hi\nfn main() {}\n", 2));
        let result = wait_for(&service).expect("result");
        assert_eq!(result.generation, 1);
        assert!(result
            .spans
            .iter()
            .any(|span| highlight_is(span, "comment")));
        assert!(result
            .spans
            .iter()
            .any(|span| highlight_is(span, "keyword")));
    }

    #[test]
    fn service_updates_after_an_edit() {
        let service = SyntaxService::new();
        service.submit(request("fn main() {}\n", 1));
        let first = wait_for(&service).expect("first result");
        assert!(first.spans.iter().any(|span| highlight_is(span, "keyword")));

        let mut edited = request("/*\nfn main() {}\n", 2);
        edited.key.revision = 2;
        service.submit(edited);
        let second = wait_for(&service).expect("second result");
        assert!(second
            .spans
            .iter()
            .any(|span| highlight_is(span, "comment") && span.row == 1));
    }

    fn highlight_is(span: &SyntaxSpan, name: &str) -> bool {
        registry().name(span.highlight) == Some(name)
    }

    fn wait_for(service: &SyntaxService) -> Option<SyntaxResult> {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if let Some(result) = service.try_recv() {
                return Some(result);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        None
    }
}
