use crate::highlight::{Document, RuntimeCache};
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
            match self.shared.runtimes.get(key.language) {
                Some(runtime) => {
                    let mut document =
                        Document::with_runtimes(key.language, self.shared.runtimes.clone());
                    document.first_paint(&request, &runtime)
                }
                None => (key.top..key.end, Vec::new()),
            }
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
        self.prepare(key);
        let Some(runtime) = self.runtimes.get(key.language) else {
            return (key.top..key.end, Vec::new());
        };
        let Some(document) = self.document.as_mut() else {
            return (key.top..key.end, Vec::new());
        };
        document.update(request, &runtime)
    }

    fn prepare(&mut self, key: SyntaxKey) {
        if self.document_id == Some(key.document) && self.language == Some(key.language) {
            return;
        }
        self.document = Some(Document::with_runtimes(key.language, self.runtimes.clone()));
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
    use crate::span::SyntaxEdit;

    fn edit_request(
        revision: u64,
        base_revision: u64,
        text: &str,
        edits: Vec<SyntaxEdit>,
    ) -> SyntaxRequest {
        SyntaxRequest {
            base_revision,
            edits,
            key: SyntaxKey {
                document: 1,
                revision,
                top: 0,
                end: text.lines().count(),
                language: LanguageId::Rust,
            },
            text: Rope::from_str(text),
        }
    }

    fn insertion(start: usize, text: &str) -> SyntaxEdit {
        SyntaxEdit {
            start,
            end: start,
            text: text.to_string(),
        }
    }

    fn request(text: &str, rows: usize) -> SyntaxRequest {
        SyntaxRequest {
            base_revision: 0,
            edits: Vec::new(),
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
            .any(|span| highlight_is(span, "keyword.function")));
    }

    #[test]
    fn synchronous_highlight_does_not_wait_for_the_worker() {
        let service = SyntaxService::new();
        let result = service.highlight_now(request("// hi\nfn main() {}\n", 2));
        assert!(result
            .spans
            .iter()
            .any(|span| highlight_is(span, "comment")));
        assert!(result
            .spans
            .iter()
            .any(|span| highlight_is(span, "keyword.function")));
        assert!(result.generation > 0);
    }

    #[test]
    fn service_receives_a_result_with_a_timeout() {
        let service = SyntaxService::new();
        service.submit(request("fn main() {}\n", 1));
        let result = service
            .recv_timeout(Duration::from_secs(5))
            .expect("result");
        assert_eq!(result.key.revision, 1);
    }

    #[test]
    fn service_reports_an_empty_timeout() {
        let service = SyntaxService::new();
        assert!(service.recv_timeout(Duration::ZERO).is_none());
    }

    #[test]
    fn service_updates_after_an_edit() {
        let service = SyntaxService::new();
        service.submit(request("fn main() {}\n", 1));
        let first = wait_for(&service).expect("first result");
        assert!(first
            .spans
            .iter()
            .any(|span| highlight_is(span, "keyword.function")));

        let mut edited = request("/*\nfn main() {}\n", 2);
        edited.key.revision = 2;
        service.submit(edited);
        let second = wait_for(&service).expect("second result");
        assert!(second
            .spans
            .iter()
            .any(|span| highlight_is(span, "comment") && span.row == 1));
    }

    #[test]
    fn service_applies_incremental_edits() {
        let service = SyntaxService::new();
        service.submit(edit_request(1, 1, "fn main() {}\n", Vec::new()));
        wait_for(&service).expect("first result");

        let edited = "// note\nfn main() {}\n";
        service.submit(edit_request(2, 1, edited, vec![insertion(0, "// note\n")]));
        let second = wait_for(&service).expect("second result");
        assert!(second
            .spans
            .iter()
            .any(|span| highlight_is(span, "comment") && span.row == 0));
    }

    #[test]
    fn queued_requests_merge_when_edits_chain() {
        let mut queued = Queued {
            generation: 1,
            request: edit_request(1, 0, "a", vec![insertion(0, "a")]),
        };
        let next = edit_request(2, 1, "ab", vec![insertion(1, "b")]);
        assert!(can_merge(&queued, &next));
        merge_request(&mut queued, 2, next);
        assert_eq!(queued.generation, 2);
        assert_eq!(queued.request.base_revision, 0);
        assert_eq!(queued.request.key.revision, 2);
        assert_eq!(queued.request.edits.len(), 2);
        assert_eq!(queued.request.text.to_string(), "ab");
    }

    #[test]
    fn queued_requests_without_edits_do_not_merge() {
        let queued = Queued {
            generation: 1,
            request: edit_request(1, 0, "a", Vec::new()),
        };
        let next = edit_request(2, 1, "ab", vec![insertion(1, "b")]);
        assert!(!can_merge(&queued, &next));
    }

    #[test]
    fn queued_requests_do_not_merge_across_a_base_gap() {
        let queued = Queued {
            generation: 1,
            request: edit_request(1, 0, "a", vec![insertion(0, "a")]),
        };
        let next = edit_request(2, 2, "ab", vec![insertion(1, "b")]);
        assert!(!can_merge(&queued, &next));
    }

    #[test]
    fn queued_requests_do_not_merge_across_documents_or_languages() {
        let queued = Queued {
            generation: 1,
            request: edit_request(1, 0, "a", vec![insertion(0, "a")]),
        };
        let mut other_document = edit_request(2, 1, "ab", vec![insertion(1, "b")]);
        other_document.key.document = 2;
        assert!(!can_merge(&queued, &other_document));

        let mut other_language = edit_request(2, 1, "ab", vec![insertion(1, "b")]);
        other_language.key.language = LanguageId::Python;
        assert!(!can_merge(&queued, &other_language));
    }

    #[test]
    fn queued_requests_merge_viewport_only_updates() {
        let mut queued = Queued {
            generation: 1,
            request: edit_request(1, 0, "a", vec![insertion(0, "a")]),
        };
        let next = edit_request(1, 1, "a", Vec::new());
        assert!(can_merge(&queued, &next));
        merge_request(&mut queued, 2, next);
        assert_eq!(queued.request.edits.len(), 1);
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
