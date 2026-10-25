mod actions;
mod view;

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use kawi_git::{BranchStatus, Status, StatusService};
use kawi_tui::statusbar::GitStatusInfo;
use kawi_watch::WatchService;

use super::App;

const SHORT_OID: usize = 7;

#[derive(Default)]
pub(super) struct GitStatusState {
    service: Option<StatusService>,
    status: Option<Status>,
    generation: u64,
    loading: bool,
    pending_root: Option<PathBuf>,
    root: Option<PathBuf>,
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

    pub(super) fn display(&self) -> Option<GitStatusInfo<'_>> {
        let status = self.status.as_ref()?;
        let branch = match &status.branch {
            BranchStatus::Named { name, .. } => {
                if name.is_empty() {
                    "HEAD"
                } else {
                    name
                }
            }
            BranchStatus::Detached { oid } => &oid[..oid.len().min(SHORT_OID)],
        };
        let upstream = status.upstream();
        Some(GitStatusInfo {
            branch,
            operation: status.operation.label(),
            ahead: upstream.map_or(0, |upstream| upstream.ahead),
            behind: upstream.map_or(0, |upstream| upstream.behind),
        })
    }

    pub(super) fn refresh(&mut self, root: PathBuf, wake: Sender<()>) {
        self.root = Some(root.clone());
        self.generation = self.generation.saturating_add(1);
        if self.loading {
            self.pending_root = Some(root);
            return;
        }
        self.start_request(root, wake);
    }

    fn start_request(&mut self, root: PathBuf, wake: Sender<()>) {
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
        if let Err(error) = service.request(generation, root, wake) {
            self.last_error = Some(error.to_string());
            return;
        }
        self.loading = true;
    }

    pub(super) fn sync(&mut self, wake: Sender<()>) -> bool {
        let mut changed = false;
        let mut events = Vec::new();
        if let Some(service) = self.service.as_ref() {
            while let Some(event) = service.try_recv() {
                events.push(event);
            }
        }
        for event in events {
            self.loading = false;
            if event.generation != self.generation {
                continue;
            }
            match event.result {
                Ok(Some(status)) => {
                    self.apply_status(status, &wake);
                    changed = true;
                }
                Ok(None) => {
                    if self.status.take().is_some() {
                        changed = true;
                    }
                    self.watcher = None;
                    self.watched_git_dir = None;
                    self.last_error = None;
                }
                Err(error) => {
                    self.last_error = Some(error.to_string());
                    changed = true;
                }
            }
        }
        if !self.loading {
            if let Some(root) = self.pending_root.take() {
                self.start_request(root, wake);
            }
        }
        changed
    }

    fn apply_status(&mut self, status: Status, wake: &Sender<()>) {
        self.ensure_watcher(&status.git_dir, wake);
        self.last_error = None;
        let selected = self
            .view
            .as_ref()
            .and_then(view::GitStatusView::selected_key);
        if self.view.is_some() {
            self.view = Some(view::GitStatusView::new(&status, selected));
        }
        if self.pending_open {
            self.view = Some(view::GitStatusView::new(&status, None));
            self.pending_open = false;
        }
        self.status = Some(status);
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
        if let Some(root) = self.root.clone() {
            self.refresh(root, wake);
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
        if let Some(root) = self.root.clone() {
            self.refresh(root, wake);
        }
    }

    pub(super) fn error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }
}

fn is_relevant(path: &Path, git_dir: &Path) -> bool {
    if path == git_dir {
        return true;
    }
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    matches!(
        name,
        "HEAD"
            | "index"
            | "FETCH_HEAD"
            | "packed-refs"
            | "ORIG_HEAD"
            | "MERGE_HEAD"
            | "CHERRY_PICK_HEAD"
            | "REVERT_HEAD"
            | "BISECT_LOG"
    )
}

impl App {
    pub(super) fn refresh_git_status(&mut self) {
        let root = self.task_root();
        self.git_status.refresh(root, self.events.wake());
    }

    pub(super) fn sync_git_status(&mut self) -> bool {
        let wake = self.events.wake();
        self.git_status.sync(wake)
    }

    pub(super) fn poll_git_status(&mut self) -> bool {
        let wake = self.events.wake();
        self.git_status.poll(wake)
    }

    pub(super) fn open_git_status(&mut self) {
        if self
            .git_status
            .status()
            .is_some_and(|status| status.entries.is_empty())
        {
            self.show_message("Working tree clean".to_string());
            return;
        }
        let wake = self.events.wake();
        self.git_status.open(wake);
        if let Some(error) = self.git_status.error() {
            self.show_message(format!("Git status unavailable: {error}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_git::{BranchStatus, OperationState, Status, Upstream};

    fn status(branch: BranchStatus) -> Status {
        Status {
            root: Default::default(),
            git_dir: Default::default(),
            branch,
            operation: OperationState::None,
            entries: Vec::new(),
        }
    }

    #[test]
    fn display_uses_branch_and_divergence() {
        let state = GitStatusState {
            status: Some(status(BranchStatus::Named {
                name: "main".to_string(),
                oid: Some("abcdef".to_string()),
                upstream: Some(Upstream {
                    name: "origin/main".to_string(),
                    ahead: 3,
                    behind: 2,
                }),
            })),
            ..GitStatusState::default()
        };
        let display = state.display().unwrap();
        assert_eq!(display.branch, "main");
        assert_eq!(display.ahead, 3);
        assert_eq!(display.behind, 2);
        assert_eq!(display.operation, None);
    }

    #[test]
    fn display_shortens_detached_heads_and_shows_operations() {
        let mut status = status(BranchStatus::Detached {
            oid: "0123456789abcdef".to_string(),
        });
        status.operation = OperationState::Rebase;
        let state = GitStatusState {
            status: Some(status),
            ..GitStatusState::default()
        };
        let display = state.display().unwrap();
        assert_eq!(display.branch, "0123456");
        assert_eq!(display.operation, Some("REBASING"));
    }

    #[test]
    fn watcher_relevance_is_limited_to_git_state() {
        let git_dir = Path::new("/repo/.git");
        assert!(is_relevant(&git_dir.join("HEAD"), git_dir));
        assert!(is_relevant(&git_dir.join("index"), git_dir));
        assert!(is_relevant(git_dir, git_dir));
        assert!(!is_relevant(&git_dir.join("objects"), git_dir));
    }
}
