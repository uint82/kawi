use std::path::PathBuf;

use kawi_git::{FileLoadEvent, FileLoadService};

use super::git::GitSession;
use super::App;

impl App {
    pub(super) fn sync_git(&mut self) {
        let path = self
            .editor
            .has_file()
            .then(|| PathBuf::from(self.editor.filename()));
        self.sync_git_status_path(path.as_deref());
        if self.editor.current_buffer_is_readonly() {
            self.clear_git();
            return;
        }
        if path.as_deref() != self.git_path.as_deref() {
            self.clear_git();
            self.git_path = path.clone();
            if let Some(path) = path {
                self.start_git_load(path);
            }
            return;
        }
        let revision = self.editor.buffer_revision();
        let Some(session) = self.git.as_mut() else {
            return;
        };
        let buffer_no_final_newline = !self.editor.text_has_final_newline();
        let changed = if session.revision() != revision
            || session.buffer_no_final_newline() != buffer_no_final_newline
        {
            session.update(&self.editor.text_lines(), revision, buffer_no_final_newline)
        } else {
            false
        };
        if changed {
            self.update_signcolumn_width();
        }
    }

    pub(super) fn reload_git(&mut self) {
        self.clear_git();
        self.sync_git();
    }

    pub(super) fn poll_git_load(&mut self) -> bool {
        let mut events = Vec::new();
        if let Some(service) = self.git_load.as_ref() {
            while let Some(event) = service.try_recv() {
                events.push(event);
            }
        }
        let mut changed = false;
        for event in events {
            changed |= self.apply_git_load(event);
        }
        changed
    }

    fn apply_git_load(&mut self, event: FileLoadEvent) -> bool {
        if event.generation != self.git_load_generation
            || self.git_path.as_deref() != Some(event.path.as_path())
        {
            return false;
        }
        self.git_loading = false;
        match event.result {
            Ok(Some(file)) => {
                self.git = Some(GitSession::new(
                    file,
                    &self.editor.text_lines(),
                    self.editor.buffer_revision(),
                    !self.editor.text_has_final_newline(),
                ));
                self.git_error = None;
            }
            Ok(None) => {
                self.git = None;
                self.git_error = None;
            }
            Err(error) => {
                self.git = None;
                self.git_error = Some(error.to_string());
            }
        }
        self.update_signcolumn_width();
        true
    }

    fn start_git_load(&mut self, path: PathBuf) {
        self.git_load_generation = self.git_load_generation.wrapping_add(1);
        let generation = self.git_load_generation;
        if self.git_load.is_none() {
            match FileLoadService::new() {
                Ok(service) => self.git_load = Some(service),
                Err(error) => {
                    self.git_error = Some(error.to_string());
                    return;
                }
            }
        }
        let wake = self.events.wake();
        let Some(service) = self.git_load.as_ref() else {
            return;
        };
        if let Err(error) = service.request(generation, path, wake) {
            self.git_error = Some(error.to_string());
            return;
        }
        self.git_loading = true;
    }

    fn clear_git(&mut self) {
        self.git = None;
        self.git_path = None;
        self.git_error = None;
        self.git_loading = false;
        self.git_load_generation = self.git_load_generation.wrapping_add(1);
        self.update_signcolumn_width();
    }
}
