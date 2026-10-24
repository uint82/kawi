use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Context, Result};
use kawi_buffer::content_hash;
use notify::event::{AccessKind, AccessMode, EventKind};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

const DEBOUNCE: Duration = Duration::from_millis(75);
const TICK: Duration = Duration::from_millis(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fingerprint {
    modified: Option<SystemTime>,
    len: u64,
    hash: u64,
}

impl Fingerprint {
    pub fn read(path: &Path) -> io::Result<Self> {
        let metadata = fs::metadata(path)?;
        let bytes = fs::read(path)?;
        Ok(Self {
            modified: metadata.modified().ok(),
            len: bytes.len() as u64,
            hash: content_hash(&bytes),
        })
    }

    pub fn matches(&self, other: &Self) -> bool {
        self.len == other.len && self.hash == other.hash
    }

    pub fn modified(&self) -> Option<SystemTime> {
        self.modified
    }
}

pub struct WatchService {
    commands: Sender<Command>,
    events: Receiver<Vec<PathBuf>>,
    worker: Option<JoinHandle<()>>,
}

enum Command {
    Watch(BTreeSet<PathBuf>),
    Shutdown,
}

impl WatchService {
    pub fn new(wake: Sender<()>) -> Result<Self> {
        let (raw_events, raw_receiver) = mpsc::channel();
        let (commands, command_receiver) = mpsc::channel();
        let (events, event_receiver) = mpsc::channel();
        let watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
            let Ok(event) = event else {
                return;
            };
            if !interesting(&event.kind) {
                return;
            }
            let _ = raw_events.send(event.paths);
        })
        .context("create file watcher")?;
        let worker = thread::Builder::new()
            .name("kawi-watch".to_string())
            .spawn(move || {
                run_worker(watcher, raw_receiver, command_receiver, events, wake);
            })
            .context("spawn file watcher")?;
        Ok(Self {
            commands,
            events: event_receiver,
            worker: Some(worker),
        })
    }

    pub fn watch_directories<'a>(&self, directories: impl IntoIterator<Item = &'a Path>) {
        let watched = directories.into_iter().map(Path::to_path_buf).collect();
        let _ = self.commands.send(Command::Watch(watched));
    }

    pub fn try_recv_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        while let Ok(batch) = self.events.try_recv() {
            paths.extend(batch);
        }
        paths.sort();
        paths.dedup();
        paths
    }
}

impl Drop for WatchService {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run_worker(
    mut watcher: RecommendedWatcher,
    events: Receiver<Vec<PathBuf>>,
    commands: Receiver<Command>,
    output: Sender<Vec<PathBuf>>,
    wake: Sender<()>,
) {
    let mut watched = BTreeSet::new();
    let mut pending = BTreeSet::new();
    let mut last_event = None;
    loop {
        while let Ok(command) = commands.try_recv() {
            match command {
                Command::Watch(parents) => sync_watches(&mut watcher, &mut watched, &parents),
                Command::Shutdown => return,
            }
        }
        match events.recv_timeout(TICK) {
            Ok(paths) => {
                pending.extend(paths);
                last_event = Some(Instant::now());
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        let Some(seen) = last_event else {
            continue;
        };
        if seen.elapsed() < DEBOUNCE || pending.is_empty() {
            continue;
        }
        let batch = std::mem::take(&mut pending);
        last_event = None;
        if output.send(batch.into_iter().collect()).is_ok() {
            let _ = wake.send(());
        }
    }
}

fn sync_watches(
    watcher: &mut RecommendedWatcher,
    watched: &mut BTreeSet<PathBuf>,
    parents: &BTreeSet<PathBuf>,
) {
    for path in watched.difference(parents) {
        let _ = watcher.unwatch(path);
    }
    let missing: Vec<PathBuf> = parents.difference(watched).cloned().collect();
    for path in missing {
        if path.is_dir() && watcher.watch(&path, RecursiveMode::NonRecursive).is_ok() {
            watched.insert(path);
        }
    }
    watched.retain(|path| parents.contains(path));
}

fn interesting(kind: &EventKind) -> bool {
    match kind {
        EventKind::Access(AccessKind::Close(AccessMode::Write)) => true,
        EventKind::Access(_) => false,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let dir = std::env::temp_dir().join(format!(
            "kawi_watch_{}_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
            name
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("directory");
        dir
    }

    #[test]
    fn fingerprints_match_identical_content() {
        let dir = temp_dir("fingerprint");
        let path = dir.join("a.txt");
        fs::write(&path, "alpha\n").expect("write");
        let first = Fingerprint::read(&path).expect("first");
        fs::write(&path, "alpha\n").expect("rewrite");
        let second = Fingerprint::read(&path).expect("second");
        assert!(first.matches(&second));
        assert!(first.modified().is_some());
        fs::write(&path, "beta\n").expect("change");
        let third = Fingerprint::read(&path).expect("third");
        assert!(!first.matches(&third));
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn watcher_reports_writes_and_renames() {
        let dir = temp_dir("events");
        let path = dir.join("a.txt");
        let temp = dir.join("a.tmp");
        fs::write(&path, "alpha\n").expect("write");
        let (wake, _wake_rx) = mpsc::channel();
        let service = WatchService::new(wake).expect("service");
        service.watch_directories([dir.as_path()]);
        thread::sleep(Duration::from_millis(100));
        fs::write(&temp, "beta\n").expect("temp");
        fs::rename(&temp, &path).expect("rename");
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut seen = false;
        while Instant::now() < deadline {
            if service.try_recv_paths().iter().any(|event| event == &path) {
                seen = true;
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(seen, "rename target was not reported");
        drop(service);
        fs::remove_dir_all(dir).expect("cleanup");
    }
}
