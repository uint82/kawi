use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use kawi_watch::WatchService;

use super::session::DirectoryBuffers;
use super::verify::EXTERNAL_CHANGE_MESSAGE;
use roots::{root_identity, root_identity_changed, watched_paths, RootIdentity};

mod roots;
use crate::app::App;

pub(super) struct DirectoryWatch {
    service: WatchService,
    roots: BTreeSet<PathBuf>,
    identities: BTreeMap<PathBuf, Option<RootIdentity>>,
}

impl DirectoryWatch {
    fn new(wake: Sender<()>) -> Result<Self, String> {
        WatchService::new(wake)
            .map(|service| Self {
                service,
                roots: BTreeSet::new(),
                identities: BTreeMap::new(),
            })
            .map_err(|error| error.to_string())
    }

    fn matches<'a>(&self, roots: impl IntoIterator<Item = &'a PathBuf>) -> bool {
        let mut count = 0;
        for root in roots {
            if !self.roots.contains(root) {
                return false;
            }
            count += 1;
        }
        count == self.roots.len()
    }

    fn sync(&mut self, roots: BTreeSet<PathBuf>) {
        let watched = watched_paths(&roots);
        self.service
            .watch_directories(watched.iter().map(PathBuf::as_path));
        self.identities = roots
            .iter()
            .map(|root| (root.clone(), root_identity(root)))
            .collect();
        self.roots = roots;
    }

    fn root_changed(&self, paths: &[PathBuf]) -> bool {
        paths.iter().any(|path| {
            self.roots.contains(path)
                && root_identity_changed(path, self.identities.get(path).copied().flatten())
        })
    }

    fn invalidate(&mut self) {
        self.roots.clear();
        self.identities.clear();
    }

    fn paths(&self) -> Vec<PathBuf> {
        self.service.try_recv_paths()
    }
}

impl DirectoryBuffers {
    pub(in crate::app) fn set_wake(&mut self, wake: Sender<()>) {
        self.wake = Some(wake);
    }

    pub(in crate::app) fn sync_watch(&mut self) -> Option<String> {
        if self.states.is_empty() {
            self.watch = None;
            self.watch_failed = false;
            return None;
        }
        if self.watch.is_none() {
            if self.watch_failed {
                return None;
            }
            let wake = self.wake.clone()?;
            match DirectoryWatch::new(wake) {
                Ok(watch) => self.watch = Some(watch),
                Err(error) => {
                    self.watch_failed = true;
                    return Some(format!("Directory watch unavailable: {error}"));
                }
            }
        }
        let matches = self
            .watch
            .as_ref()
            .is_some_and(|watch| watch.matches(self.states.values().map(|state| &state.root)));
        if matches {
            return None;
        }
        let roots = self
            .states
            .values()
            .map(|state| state.root.clone())
            .collect();
        if let Some(watch) = self.watch.as_mut() {
            watch.sync(roots);
        }
        None
    }

    pub(in crate::app) fn watch_paths(&self) -> Vec<PathBuf> {
        self.watch
            .as_ref()
            .map(DirectoryWatch::paths)
            .unwrap_or_default()
    }

    pub(in crate::app) fn watch_root_changed(&self, paths: &[PathBuf]) -> bool {
        self.watch
            .as_ref()
            .is_some_and(|watch| watch.root_changed(paths))
    }

    pub(in crate::app) fn invalidate_watch(&mut self) {
        if let Some(watch) = self.watch.as_mut() {
            watch.invalidate();
        }
    }

    pub(in crate::app) fn matching_roots(&self, paths: &[PathBuf]) -> Vec<usize> {
        self.states
            .iter()
            .filter(|(_, state)| {
                paths
                    .iter()
                    .any(|path| path.starts_with(state.root.as_path()))
            })
            .map(|(number, _)| *number)
            .collect()
    }
}

impl App {
    pub(in crate::app) fn sync_directory_watch(&mut self) {
        if let Some(message) = self.directory.sync_watch() {
            self.show_message(message);
        }
    }

    pub(in crate::app) fn poll_directory_watch(&mut self) -> bool {
        let paths = self.directory.watch_paths();
        self.handle_directory_watch_paths(&paths)
    }

    pub(in crate::app) fn handle_directory_watch_paths(&mut self, paths: &[PathBuf]) -> bool {
        if paths.is_empty() {
            return false;
        }
        let root_changed = self.directory.watch_root_changed(paths);
        let mut redraw = false;
        for number in self.directory.matching_roots(paths) {
            redraw |= self.refresh_directory_from_watch(number);
        }
        if root_changed {
            self.directory.invalidate_watch();
        }
        redraw
    }

    fn refresh_directory_from_watch(&mut self, number: usize) -> bool {
        if self.directory_loading.is_loading(number) {
            return false;
        }
        if self.editor.buffer_is_dirty(number).unwrap_or(false) {
            if self.directory.mark_external_change(number) {
                self.show_message(EXTERNAL_CHANGE_MESSAGE.to_string());
            }
            return true;
        }
        let focus = self.directory_focus_text(number);
        match self.start_directory_watch_reload(number) {
            Ok(true) => {
                if let Some(text) = focus {
                    self.directory_loading.defer_focus(number, text);
                }
                true
            }
            Ok(false) => false,
            Err(message) => {
                if number == self.editor.buffer_number() {
                    self.show_message(message);
                }
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::mpsc;

    use super::*;

    #[test]
    fn ordinary_changes_keep_watch_roots_registered() {
        let dir = crate::test_support::temp_dir("directory_watch_registered");
        let (wake, _events) = mpsc::channel();
        let mut watch = DirectoryWatch::new(wake).unwrap();
        watch.sync(BTreeSet::from([dir.clone()]));
        fs::write(dir.join("alpha.txt"), "content\n").unwrap();
        assert!(!watch.root_changed(std::slice::from_ref(&dir)));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn replacement_marks_watch_roots_for_rearming() {
        let dir = crate::test_support::temp_dir("directory_watch_rearm");
        let replacement = crate::test_support::temp_dir("directory_watch_rearm_new");
        let (wake, _events) = mpsc::channel();
        let mut watch = DirectoryWatch::new(wake).unwrap();
        watch.sync(BTreeSet::from([dir.clone()]));
        fs::remove_dir_all(&dir).unwrap();
        fs::rename(&replacement, &dir).unwrap();
        assert!(watch.root_changed(std::slice::from_ref(&dir)));
        watch.invalidate();
        assert!(!watch.root_changed(std::slice::from_ref(&dir)));
        fs::remove_dir_all(dir).unwrap();
    }
}
