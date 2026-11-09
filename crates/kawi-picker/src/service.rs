use crate::files::{relative_path, walk_project};
use crate::grep::{grep_project_while, GrepHit};
use crate::replace::{replace_preview_while, ReplacePreview};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectRequest {
    Files {
        root: PathBuf,
    },
    Grep {
        root: PathBuf,
        query: String,
        limit: usize,
    },
    Replace {
        root: PathBuf,
        query: String,
        replacement: String,
        limit: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectResult {
    Files {
        id: u64,
        files: Vec<String>,
    },
    Grep {
        id: u64,
        hits: Vec<GrepHit>,
        error: Option<String>,
    },
    Replace {
        id: u64,
        preview: Option<ReplacePreview>,
        error: Option<String>,
    },
}

pub struct ProjectService {
    shared: Arc<Shared>,
    results: Receiver<ProjectResult>,
    worker: Option<JoinHandle<()>>,
}

struct Shared {
    request: Mutex<Option<Queued>>,
    ready: Condvar,
    shutdown: AtomicBool,
    generation: AtomicU64,
}

struct Queued {
    id: u64,
    request: ProjectRequest,
}

impl ProjectService {
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
                .name("kawi-project".to_string())
                .spawn(move || run(shared, sender, wake))
                .ok()
        };
        Self {
            shared,
            results,
            worker,
        }
    }

    pub fn submit(&self, request: ProjectRequest) -> u64 {
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
        *slot = Some(Queued { id, request });
        self.shared.ready.notify_one();
        id
    }

    pub fn try_recv(&self) -> Option<ProjectResult> {
        self.results.try_recv().ok()
    }
}

impl Default for ProjectService {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ProjectService {
    fn drop(&mut self) {
        {
            let _slot = self
                .shared
                .request
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            self.shared.shutdown.store(true, Ordering::SeqCst);
        }
        self.shared.generation.fetch_add(1, Ordering::SeqCst);
        self.shared.ready.notify_one();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run(shared: Arc<Shared>, sender: Sender<ProjectResult>, wake: Sender<()>) {
    while let Some(queued) = next_request(&shared) {
        let result = process(&shared, queued);
        if shared.generation.load(Ordering::SeqCst) != result.id() {
            continue;
        }
        if sender.send(result).is_err() {
            return;
        }
        let _ = wake.send(());
    }
}

fn process(shared: &Shared, queued: Queued) -> ProjectResult {
    match queued.request {
        ProjectRequest::Files { root } => ProjectResult::Files {
            id: queued.id,
            files: scan_files(shared, queued.id, &root),
        },
        ProjectRequest::Grep { root, query, limit } => {
            match grep_project_while(&root, &query, limit, || {
                shared.generation.load(Ordering::SeqCst) == queued.id
            }) {
                Ok(hits) => ProjectResult::Grep {
                    id: queued.id,
                    hits,
                    error: None,
                },
                Err(error) => ProjectResult::Grep {
                    id: queued.id,
                    hits: Vec::new(),
                    error: Some(error),
                },
            }
        }
        ProjectRequest::Replace {
            root,
            query,
            replacement,
            limit,
        } => match replace_preview_while(&root, &query, &replacement, limit, || {
            shared.generation.load(Ordering::SeqCst) == queued.id
        }) {
            Ok(preview) => ProjectResult::Replace {
                id: queued.id,
                preview: Some(preview),
                error: None,
            },
            Err(error) => ProjectResult::Replace {
                id: queued.id,
                preview: None,
                error: Some(error),
            },
        },
    }
}

fn scan_files(shared: &Shared, id: u64, root: &std::path::Path) -> Vec<String> {
    let mut files = Vec::new();
    walk_project(root, |path| {
        if shared.generation.load(Ordering::SeqCst) != id {
            return false;
        }
        if let Some(relative) = relative_path(root, path) {
            files.push(relative);
        }
        true
    });
    files.sort();
    files
}

impl ProjectResult {
    pub fn id(&self) -> u64 {
        match self {
            Self::Files { id, .. } | Self::Grep { id, .. } | Self::Replace { id, .. } => *id,
        }
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::*;

    fn project(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kawi_project_{name}_{stamp}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("project");
        fs::write(root.join("alpha.txt"), "needle\n").expect("alpha");
        root
    }

    fn wait_for(service: &ProjectService) -> ProjectResult {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            if let Some(result) = service.try_recv() {
                return result;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("project result timed out");
    }

    #[test]
    fn service_notifies_the_wake_channel() {
        let (wake, wakes) = mpsc::channel();
        let service = ProjectService::with_wake(wake);
        service.submit(ProjectRequest::Files {
            root: project("wake"),
        });
        wakes
            .recv_timeout(Duration::from_secs(5))
            .expect("wake notification");
        assert!(service.try_recv().is_some());
    }

    #[test]
    fn service_lists_files() {
        let root = project("files");
        let service = ProjectService::new();
        let id = service.submit(ProjectRequest::Files { root: root.clone() });
        match wait_for(&service) {
            ProjectResult::Files { id: result, files } => {
                assert_eq!(result, id);
                assert_eq!(files, vec!["alpha.txt"]);
            }
            other => panic!("unexpected result: {other:?}"),
        }
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn service_builds_replace_previews() {
        let root = project("replace");
        let service = ProjectService::new();
        let id = service.submit(ProjectRequest::Replace {
            root: root.clone(),
            query: "needle".to_string(),
            replacement: "thread".to_string(),
            limit: 10,
        });
        match wait_for(&service) {
            ProjectResult::Replace {
                id: result,
                preview,
                error,
            } => {
                assert_eq!(result, id);
                assert_eq!(error, None);
                let preview = preview.expect("preview");
                assert_eq!(preview.matches, 1);
                assert_eq!(preview.files[0].content, "thread\n");
            }
            other => panic!("unexpected result: {other:?}"),
        }
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn service_searches_files() {
        let root = project("grep");
        let service = ProjectService::new();
        let id = service.submit(ProjectRequest::Grep {
            root: root.clone(),
            query: "needle".to_string(),
            limit: 10,
        });
        match wait_for(&service) {
            ProjectResult::Grep {
                id: result,
                hits,
                error,
            } => {
                assert_eq!(result, id);
                assert_eq!(error, None);
                assert_eq!(hits.len(), 1);
                assert_eq!(hits[0].path, "alpha.txt");
            }
            other => panic!("unexpected result: {other:?}"),
        }
        fs::remove_dir_all(root).expect("cleanup");
    }
}
