use std::path::{Path, PathBuf};

use super::super::App;
use super::path::absolute_path;

impl App {
    pub(in crate::app) fn refresh_git_status(&mut self) {
        let path = self
            .editor
            .has_file()
            .then(|| PathBuf::from(self.editor.filename()));
        self.git_status_path = path.clone();
        match path {
            Some(path) => self
                .git_status
                .refresh(absolute_path(&path), self.events.wake()),
            None => self.git_status.clear(),
        }
    }

    pub(in crate::app) fn sync_git_status_path(&mut self, path: Option<&Path>) {
        if path == self.git_status_path.as_deref() {
            return;
        }
        self.refresh_git_status();
    }

    pub(in crate::app) fn sync_git_status(&mut self) -> bool {
        let wake = self.events.wake();
        self.git_status.sync(wake)
    }

    pub(in crate::app) fn poll_git_status(&mut self) -> bool {
        let wake = self.events.wake();
        self.git_status.poll(wake)
    }

    pub(in crate::app) fn open_git_status(&mut self) {
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
