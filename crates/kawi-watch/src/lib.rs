use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use kawi_buffer::content_hash;
use notify::event::{AccessKind, AccessMode, EventKind};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

mod clock;
mod debounce;

#[cfg(test)]
mod tests;

use clock::WatchOptions;
use debounce::Debounce;

const DEBOUNCE: Duration = Duration::from_millis(75);
const TICK: Duration = Duration::from_millis(10);
const WATCH_READY_TIMEOUT: Duration = Duration::from_secs(2);

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
    Watch {
        directories: BTreeSet<PathBuf>,
        ready: Sender<()>,
    },
    Shutdown,
}

impl WatchService {
    pub fn new(wake: Sender<()>) -> Result<Self> {
        Self::with_options(wake, WatchOptions::new(DEBOUNCE, TICK))
    }

    fn with_options(wake: Sender<()>, options: WatchOptions) -> Result<Self> {
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
                run_worker(
                    watcher,
                    raw_receiver,
                    command_receiver,
                    events,
                    wake,
                    options,
                );
            })
            .context("spawn file watcher")?;
        Ok(Self {
            commands,
            events: event_receiver,
            worker: Some(worker),
        })
    }

    pub fn watch_directories<'a>(&self, directories: impl IntoIterator<Item = &'a Path>) {
        let directories = directories.into_iter().map(Path::to_path_buf).collect();
        let (ready, installed) = mpsc::channel();
        if self
            .commands
            .send(Command::Watch { directories, ready })
            .is_ok()
        {
            let _ = installed.recv_timeout(WATCH_READY_TIMEOUT);
        }
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
    options: WatchOptions,
) {
    let mut watched = BTreeSet::new();
    let mut debounce = Debounce::new(options.debounce);
    loop {
        while let Ok(command) = commands.try_recv() {
            match command {
                Command::Watch { directories, ready } => {
                    sync_watches(&mut watcher, &mut watched, &directories);
                    let _ = ready.send(());
                }
                Command::Shutdown => return,
            }
        }
        let now = options.clock.now();
        let timeout = debounce.wait_timeout(now, options.tick);
        match events.recv_timeout(timeout) {
            Ok(paths) => debounce.push(paths, options.clock.now()),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        let Some(batch) = debounce.take_ready(options.clock.now()) else {
            continue;
        };
        if output.send(batch).is_ok() {
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
