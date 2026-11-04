use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use crate::{repo, GitFile};

pub struct FileLoadEvent {
    pub generation: u64,
    pub path: PathBuf,
    pub result: Result<Option<GitFile>>,
}

pub struct FileLoadService {
    requests: Option<Sender<Request>>,
    events: Receiver<FileLoadEvent>,
    worker: Option<JoinHandle<()>>,
}

struct Request {
    generation: u64,
    path: PathBuf,
    wake: Sender<()>,
}

impl FileLoadService {
    pub fn new() -> Result<Self> {
        let (requests, request_receiver) = mpsc::channel();
        let (event_sender, events) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("kawi-git-load".to_string())
            .spawn(move || run(request_receiver, event_sender))
            .context("spawn git load worker")?;
        Ok(Self {
            requests: Some(requests),
            events,
            worker: Some(worker),
        })
    }

    pub fn request(&self, generation: u64, path: PathBuf, wake: Sender<()>) -> Result<()> {
        self.requests
            .as_ref()
            .context("git load service closed")?
            .send(Request {
                generation,
                path,
                wake,
            })
            .context("send git load request")
    }

    pub fn try_recv(&self) -> Option<FileLoadEvent> {
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

impl Drop for FileLoadService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn run(requests: Receiver<Request>, events: Sender<FileLoadEvent>) {
    while let Ok(mut request) = requests.recv() {
        while let Ok(next) = requests.try_recv() {
            request = next;
        }
        let result = repo::load(&request.path);
        if events
            .send(FileLoadEvent {
                generation: request.generation,
                path: request.path,
                result,
            })
            .is_err()
        {
            return;
        }
        let _ = request.wake.send(());
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    use super::*;

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    fn repo(name: &str) -> PathBuf {
        let suffix = NEXT.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "kawi_git_load_{}_{}_{}",
            std::process::id(),
            name,
            suffix
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("directory");
        git(&dir, &["init", "-q"]);
        git(&dir, &["config", "user.email", "test@example.com"]);
        git(&dir, &["config", "user.name", "Test"]);
        dir
    }

    fn git(dir: &Path, args: &[&str]) {
        let output = Command::new("git")
            .current_dir(dir)
            .args(args)
            .output()
            .expect("git");
        assert!(output.status.success(), "git {args:?} failed");
    }

    #[test]
    fn loads_a_file_on_the_worker() {
        let dir = repo("file");
        let path = dir.join("a.txt");
        fs::write(&path, "alpha\n").expect("write");
        let (wake, _wake_rx) = mpsc::channel();
        let service = FileLoadService::new().expect("service");
        service.request(7, path.clone(), wake).expect("request");
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if let Some(event) = service.try_recv() {
                assert_eq!(event.generation, 7);
                assert_eq!(event.path, path);
                let file = event.result.expect("result").expect("file");
                assert!(file.untracked);
                let _ = fs::remove_dir_all(dir);
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("load timed out");
    }
}
