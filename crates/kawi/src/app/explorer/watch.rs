use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use kawi_watch::WatchService;

use super::ExplorerState;

pub(super) struct ExplorerWatch {
    service: WatchService,
}

impl ExplorerWatch {
    pub(super) fn new(wake: Sender<()>) -> Result<Self, String> {
        WatchService::new(wake)
            .map(|service| Self { service })
            .map_err(|error| error.to_string())
    }

    pub(super) fn sync<'a>(&self, directories: impl IntoIterator<Item = &'a Path>) {
        self.service.watch_directories(directories);
    }

    pub(super) fn paths(&self) -> Vec<PathBuf> {
        self.service.try_recv_paths()
    }
}

impl ExplorerState {
    pub(crate) fn sync_watch(&mut self) {
        let (Some(root), true) = (self.root.as_ref(), self.visible) else {
            self.watch = None;
            self.watch_error = None;
            return;
        };
        if self.watch.is_none() {
            let Some(wake) = self.wake.clone() else {
                return;
            };
            match ExplorerWatch::new(wake) {
                Ok(watch) => self.watch = Some(watch),
                Err(error) => {
                    self.watch_error = Some(format!("File watch unavailable: {error}"));
                    return;
                }
            }
            self.watch_error = None;
        }
        let Some(watch) = self.watch.as_ref() else {
            return;
        };
        let directories = watched_directories(root, &self.expanded);
        watch.sync(directories.iter().map(PathBuf::as_path));
    }

    pub(crate) fn poll_watch(&mut self) -> bool {
        if !self.visible {
            return false;
        }
        let Some(root) = self.root.clone() else {
            return false;
        };
        let Some(watch) = self.watch.as_ref() else {
            return false;
        };
        let paths = watch.paths();
        if paths.is_empty() || !paths.iter().any(|path| path.starts_with(&root)) {
            return false;
        }
        self.refresh();
        self.sync_watch();
        true
    }
}

pub(super) fn watched_directories(root: &Path, expanded: &BTreeSet<PathBuf>) -> BTreeSet<PathBuf> {
    let mut directories = BTreeSet::new();
    directories.insert(root.to_path_buf());
    directories.extend(expanded.iter().cloned());
    directories
}
