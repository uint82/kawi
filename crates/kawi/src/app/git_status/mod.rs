mod actions;
mod display;
mod integration;
mod path;
mod view;
mod watcher;

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use kawi_git::{Status, StatusEvent, StatusService};
use kawi_watch::WatchService;

use path::status_matches_path;
use watcher::is_relevant;

#[derive(Default)]
pub(super) struct GitStatusState {
    service: Option<StatusService>,
    status: Option<Status>,
    generation: u64,
    loading: bool,
    pending_path: Option<PathBuf>,
    path: Option<PathBuf>,
    watcher: Option<WatchService>,
    watched_git_dir: Option<PathBuf>,
    pending_open: bool,
    view: Option<view::GitStatusView>,
    last_error: Option<String>,
}

impl GitStatusState {
    pub(super) fn status(&self) -> Option<&Status> {
        self.status.as_ref()
    }

    pub(super) fn has_view(&self) -> bool {
        self.view.is_some()
    }

    pub(super) fn view_lines(&self) -> Vec<String> {
        self.view
            .as_ref()
            .map(view::GitStatusView::lines)
            .unwrap_or_default()
    }

    pub(super) fn view_selected(&self) -> Option<usize> {
        self.view.as_ref().and_then(view::GitStatusView::selected)
    }

    pub(super) fn view_status(&self) -> Option<&str> {
        self.view.as_ref().map(view::GitStatusView::status)
    }

    pub(in crate::app::git_status) fn view(&self) -> Option<&view::GitStatusView> {
        self.view.as_ref()
    }

    pub(in crate::app::git_status) fn view_mut(&mut self) -> Option<&mut view::GitStatusView> {
        self.view.as_mut()
    }

    pub(super) fn close_view(&mut self) {
        self.view = None;
        self.pending_open = false;
    }

    pub(super) fn refresh(&mut self, path: PathBuf, wake: Sender<()>) {
        if self.path.as_deref() != Some(path.as_path()) {
            self.discard_status();
        }
        self.path = Some(path.clone());
        self.generation = self.generation.saturating_add(1);
        if self.loading {
            self.pending_path = Some(path);
            return;
        }
        self.start_request(path, wake);
    }

    fn start_request(&mut self, path: PathBuf, wake: Sender<()>) {
        if self.service.is_none() {
            match StatusService::new() {
                Ok(service) => self.service = Some(service),
                Err(error) => {
                    self.last_error = Some(error.to_string());
                    return;
                }
            }
        }
        let generation = self.generation;
        let Some(service) = self.service.as_ref() else {
            return;
        };
        if let Err(error) = service.request(generation, path, wake) {
            self.last_error = Some(error.to_string());
            return;
        }
        self.loading = true;
    }

    pub(super) fn sync(&mut self, wake: Sender<()>) -> bool {
        let mut changed = false;
        for event in self.drain_events() {
            changed |= self.apply_event(event, &wake);
        }
        self.start_pending(wake);
        changed
    }

    fn drain_events(&self) -> Vec<StatusEvent> {
        let Some(service) = self.service.as_ref() else {
            return Vec::new();
        };
        let mut events = Vec::new();
        while let Some(event) = service.try_recv() {
            events.push(event);
        }
        events
    }

    fn apply_event(&mut self, event: StatusEvent, wake: &Sender<()>) -> bool {
        self.loading = false;
        if event.generation != self.generation {
            return false;
        }
        match event.result {
            Ok(Some(status)) => {
                self.apply_status(status, wake);
                true
            }
            Ok(None) => self.discard_status(),
            Err(error) => {
                self.last_error = Some(error.to_string());
                true
            }
        }
    }

    fn start_pending(&mut self, wake: Sender<()>) {
        if self.loading {
            return;
        }
        if let Some(path) = self.pending_path.take() {
            self.start_request(path, wake);
        }
    }

    fn apply_status(&mut self, status: Status, wake: &Sender<()>) {
        if !status_matches_path(self.path.as_deref(), &status) {
            self.discard_status();
            return;
        }
        self.ensure_watcher(&status.git_dir, wake);
        self.last_error = None;
        self.update_view(&status);
        self.status = Some(status);
    }

    fn update_view(&mut self, status: &Status) {
        let selected = self
            .view
            .as_ref()
            .and_then(view::GitStatusView::selected_key);
        if self.view.is_some() {
            self.view = Some(view::GitStatusView::new(status, selected));
        }
        if self.pending_open {
            self.view = Some(view::GitStatusView::new(status, None));
            self.pending_open = false;
        }
    }

    fn ensure_watcher(&mut self, git_dir: &Path, wake: &Sender<()>) {
        if self.watched_git_dir.as_deref() == Some(git_dir) {
            return;
        }
        let Ok(watcher) = WatchService::new(wake.clone()) else {
            self.watcher = None;
            self.watched_git_dir = None;
            return;
        };
        watcher.watch_directories([git_dir]);
        self.watcher = Some(watcher);
        self.watched_git_dir = Some(git_dir.to_path_buf());
    }

    pub(super) fn poll(&mut self, wake: Sender<()>) -> bool {
        let Some(watcher) = self.watcher.as_ref() else {
            return false;
        };
        let paths = watcher.try_recv_paths();
        if paths.is_empty() {
            return false;
        }
        let Some(git_dir) = self.watched_git_dir.as_deref() else {
            return false;
        };
        if !paths.iter().any(|path| is_relevant(path, git_dir)) {
            return false;
        }
        if let Some(path) = self.path.clone() {
            self.refresh(path, wake);
        }
        false
    }

    pub(super) fn open(&mut self, wake: Sender<()>) {
        if let Some(status) = self.status.as_ref() {
            if status.entries.is_empty() {
                self.view = None;
                return;
            }
            self.view = Some(view::GitStatusView::new(status, None));
            return;
        }
        self.pending_open = true;
        if let Some(path) = self.path.clone() {
            self.refresh(path, wake);
        }
    }

    pub(super) fn error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub(super) fn clear(&mut self) {
        self.discard_status();
        self.path = None;
        self.pending_path = None;
        self.generation = self.generation.saturating_add(1);
    }

    fn discard_status(&mut self) -> bool {
        let status_changed = self.status.take().is_some();
        let view_changed = self.view.take().is_some();
        self.watcher = None;
        self.watched_git_dir = None;
        self.pending_open = false;
        self.last_error = None;
        status_changed || view_changed
    }
}

#[cfg(test)]
mod tests;
