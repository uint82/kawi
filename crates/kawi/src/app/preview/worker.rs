use ropey::Rope;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

const MAX_PREVIEW_BYTES: u64 = 2 * 1024 * 1024;
const PREVIEW_CONTEXT_LINES: usize = 1_000;
const BINARY_SNIFF_BYTES: usize = 8 * 1024;

pub(super) struct PreviewFile {
    pub(super) id: u64,
    pub(super) path: PathBuf,
    pub(super) text: Option<Rope>,
    pub(super) lines: Vec<String>,
    pub(super) first_line: usize,
    pub(super) total_lines: usize,
    pub(super) focus_line: usize,
    pub(super) error: Option<String>,
}

struct PreviewRequest {
    id: u64,
    path: PathBuf,
    text: Option<Rope>,
    focus_line: usize,
}

struct Shared {
    request: Mutex<Option<PreviewRequest>>,
    ready: Condvar,
    shutdown: AtomicBool,
    generation: AtomicU64,
}

pub(super) struct PreviewService {
    shared: Arc<Shared>,
    results: Receiver<PreviewFile>,
    worker: Option<JoinHandle<()>>,
}

impl PreviewService {
    pub(super) fn with_wake(wake: Sender<()>) -> Self {
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
                .name("kawi-preview".to_string())
                .spawn(move || run(shared, sender, wake))
                .ok()
        };
        Self {
            shared,
            results,
            worker,
        }
    }

    pub(super) fn submit(&self, path: PathBuf, text: Option<Rope>, focus_line: usize) -> u64 {
        let id = self
            .shared
            .generation
            .fetch_add(1, Ordering::SeqCst)
            .saturating_add(1);
        let mut slot = self
            .shared
            .request
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *slot = Some(PreviewRequest {
            id,
            path,
            text,
            focus_line,
        });
        self.shared.ready.notify_one();
        id
    }

    pub(super) fn try_recv(&self) -> Option<PreviewFile> {
        self.results.try_recv().ok()
    }

    pub(super) fn detach(&mut self) {
        self.signal_shutdown();
        self.worker.take();
    }

    fn signal_shutdown(&self) {
        self.shared.shutdown.store(true, Ordering::SeqCst);
        self.shared.ready.notify_one();
    }
}

impl Drop for PreviewService {
    fn drop(&mut self) {
        self.signal_shutdown();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run(shared: Arc<Shared>, sender: Sender<PreviewFile>, wake: Sender<()>) {
    while let Some(request) = next_request(&shared) {
        let result = load_preview(request);
        if shared.generation.load(Ordering::SeqCst) != result.id {
            continue;
        }
        if sender.send(result).is_err() {
            return;
        }
        let _ = wake.send(());
    }
}

fn next_request(shared: &Shared) -> Option<PreviewRequest> {
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

fn load_preview(request: PreviewRequest) -> PreviewFile {
    let PreviewRequest {
        id,
        path,
        text,
        focus_line,
    } = request;
    let text = match text {
        Some(text) => text,
        None => match read_file(&path) {
            Ok(text) => text,
            Err(error) => return preview_error(id, path, error),
        },
    };
    preview_text(id, path, text, focus_line)
}

fn read_file(path: &Path) -> Result<Rope, String> {
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    if metadata.len() > MAX_PREVIEW_BYTES {
        return Err("file is too large to preview".to_string());
    }
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    if bytes.iter().take(BINARY_SNIFF_BYTES).any(|byte| *byte == 0) {
        return Err("binary files cannot be previewed".to_string());
    }
    Ok(Rope::from_str(&String::from_utf8_lossy(&bytes)))
}

fn preview_error(id: u64, path: PathBuf, error: String) -> PreviewFile {
    PreviewFile {
        id,
        path,
        text: None,
        lines: Vec::new(),
        first_line: 0,
        total_lines: 0,
        focus_line: 0,
        error: Some(error),
    }
}

fn preview_text(id: u64, path: PathBuf, text: Rope, focus_line: usize) -> PreviewFile {
    let total_lines = text.len_lines();
    let focus_line = focus_line.min(total_lines.saturating_sub(1));
    let first_line = focus_line.saturating_sub(PREVIEW_CONTEXT_LINES);
    let end = focus_line
        .saturating_add(PREVIEW_CONTEXT_LINES)
        .saturating_add(1)
        .min(total_lines);
    let mut lines: Vec<String> = (first_line..end)
        .map(|line| text.line(line).to_string().trim_end().to_string())
        .collect();
    if lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    PreviewFile {
        id,
        path,
        text: Some(text),
        lines,
        first_line,
        total_lines,
        focus_line,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(path: &str, text: Option<&str>, focus: usize) -> PreviewRequest {
        PreviewRequest {
            id: 1,
            path: PathBuf::from(path),
            text: text.map(Rope::from_str),
            focus_line: focus,
        }
    }

    #[test]
    fn previews_the_open_buffer_text() {
        let result = load_preview(request("ignored.rs", Some("one\ntwo\nthree\n"), 1));
        assert_eq!(result.lines, ["one", "two", "three"]);
        assert_eq!(result.focus_line, 1);
        assert_eq!(result.total_lines, 4);
    }

    #[test]
    fn clamps_the_focus_line() {
        let result = load_preview(request("ignored.rs", Some("one\n"), 99));
        assert_eq!(result.focus_line, 1);
    }

    #[test]
    fn reports_binary_files() {
        let path = temp_path("binary");
        fs::write(&path, b"a\0b").expect("write");
        let result = load_preview(request(&path.to_string_lossy(), None, 0));
        assert_eq!(
            result.error.as_deref(),
            Some("binary files cannot be previewed")
        );
        fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn reports_oversized_files() {
        let path = temp_path("large");
        let file = fs::File::create(&path).expect("file");
        file.set_len(MAX_PREVIEW_BYTES + 1).expect("size");
        let result = load_preview(request(&path.to_string_lossy(), None, 0));
        assert_eq!(
            result.error.as_deref(),
            Some("file is too large to preview")
        );
        fs::remove_file(path).expect("cleanup");
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "kawi_preview_{name}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ))
    }
}
