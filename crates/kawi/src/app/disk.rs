use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use kawi_editor::paths;
use kawi_watch::{Fingerprint, WatchService};

use super::App;
use state::{Observation, ObservedFile, STATUS_CHANGED, STATUS_DELETED};

mod state;
pub(super) use state::DiskState;

impl App {
    pub(super) fn start_file_watch(&mut self) {
        match WatchService::new(self.events.wake()) {
            Ok(watcher) => self.file_watch = Some(watcher),
            Err(error) => self.show_message(format!("File watch unavailable: {error}")),
        }
    }

    pub(super) fn sync_watched_files(&mut self) {
        let files = self.open_files();
        let Some(parents) = self.disk.sync(&files) else {
            return;
        };
        if let Some(watcher) = self.file_watch.as_ref() {
            watcher.watch_directories(parents.iter().map(PathBuf::as_path));
        }
    }

    fn file_move_search_paths(&self) -> BTreeSet<PathBuf> {
        let mut search = self.disk.watched.clone();
        search.extend(self.directory.watch_paths());
        search
    }

    pub(super) fn poll_file_watch(&mut self) -> bool {
        let Some(watcher) = self.file_watch.as_ref() else {
            return false;
        };
        let paths = watcher.try_recv_paths();
        if paths.is_empty() {
            return false;
        }
        self.handle_disk_paths(&paths);
        true
    }

    pub(super) fn handle_disk_paths(&mut self, paths: &[PathBuf]) {
        let search = self.file_move_search_paths();
        for number in self.disk.matching_buffers(paths) {
            let Some(path) = self.disk.path(number).map(Path::to_path_buf) else {
                continue;
            };
            let previous = self.disk.fingerprint(number);
            let mut observed = ObservedFile::read(&path, previous);
            if observed.is_missing() {
                if let Some(moved) = self.disk.moved_path(number, &search) {
                    self.retarget_file(number, &path, &moved);
                    observed = ObservedFile::read(&moved, previous);
                }
            }
            match self.disk.observe(number, observed) {
                Observation::Ignored => {}
                Observation::Same => {}
                Observation::Deleted => self.report_deleted_file(number),
                Observation::Changed => self.handle_changed_file(number, observed.fingerprint),
            }
            self.close_resolved_conflict(number);
        }
    }

    fn retarget_file(&mut self, number: usize, old: &Path, new: &Path) {
        self.move_paths(old, new);
        self.disk.retarget(number, new.to_path_buf());
        self.notify_path_move(old, new);
        self.show_message(format!(
            "File moved on disk: {} to {}",
            old.display(),
            new.display()
        ));
    }

    fn handle_changed_file(&mut self, number: usize, observed: Option<Fingerprint>) {
        let dirty = self.editor.buffer_is_dirty(number).unwrap_or(false);
        if dirty {
            self.disk.mark_conflict(number);
            self.report_changed_file(number);
            return;
        }
        match self.editor.reload_buffer(number) {
            Ok(()) => {
                if let Some(fingerprint) = observed {
                    self.disk.accept(number, fingerprint);
                }
            }
            Err(error) => {
                self.disk.mark_changed(number);
                let name = self.buffer_name(number);
                self.show_message(format!("Cannot reload {name}: {error:?}"));
            }
        }
    }

    fn report_deleted_file(&mut self, number: usize) {
        let name = self.buffer_name(number);
        self.show_message(format!("Deleted on disk: {name}; buffer kept"));
    }

    fn report_changed_file(&mut self, number: usize) {
        let name = self.buffer_name(number);
        self.show_message(format!("File changed on disk: {name}"));
    }

    pub(super) fn record_saved_buffer(&mut self) {
        self.record_saved_buffer_number(self.editor.buffer_number());
    }

    pub(super) fn record_saved_buffer_number(&mut self, number: usize) {
        let Some(path) = self.editor.buffer_path(number).map(PathBuf::from) else {
            return;
        };
        let path = absolute_path(&path);
        let fingerprint = Fingerprint::read(&path).ok();
        self.disk.record_saved(number, path, fingerprint);
        self.git_commit.mark_saved(number);
        self.refresh_git_status();
    }

    pub(super) fn record_all_saved(&mut self) {
        for (number, path) in self.open_files() {
            let fingerprint = Fingerprint::read(&path).ok();
            self.disk.record_saved(number, path, fingerprint);
            self.git_commit.mark_saved(number);
        }
        self.refresh_git_status();
    }

    pub(super) fn record_reloaded_paths(&mut self, targets: &[PathBuf]) {
        for number in self.disk.matching_buffers(targets) {
            let Some(path) = self.disk.path(number).map(Path::to_path_buf) else {
                continue;
            };
            if let Ok(fingerprint) = Fingerprint::read(&path) {
                self.disk.accept(number, fingerprint);
            }
        }
    }

    pub(super) fn disk_status(&self, number: usize) -> Option<&'static str> {
        self.disk.status(number).or_else(|| {
            if self.directory.deleted(number) {
                Some(STATUS_DELETED)
            } else {
                self.directory
                    .external_changed(number)
                    .then_some(STATUS_CHANGED)
            }
        })
    }

    pub(super) fn disk_conflict(&self, number: usize) -> bool {
        self.disk.conflict(number)
    }

    pub(super) fn disk_deleted(&self, number: usize) -> bool {
        self.disk.status(number) == Some(STATUS_DELETED) || self.directory.deleted(number)
    }

    pub(super) fn has_disk_conflicts(&self) -> bool {
        self.disk.any_conflict()
    }

    pub(super) fn disk_needs_prompt(&self, number: usize) -> bool {
        self.disk.needs_prompt(number)
    }

    pub(super) fn disk_prompt_open(&mut self, number: usize) {
        self.disk.open_prompt(number);
    }

    pub(super) fn disk_dismiss_prompt(&mut self, number: usize) {
        self.disk.dismiss(number);
    }

    pub(super) fn keep_buffer_version(&mut self, number: usize) {
        self.disk.accept_current(number);
        let name = self.buffer_name(number);
        self.show_message(format!("Kept buffer version of {name}"));
    }

    pub(super) fn load_disk_version(&mut self, number: usize) -> Result<(), String> {
        self.editor
            .reload_buffer(number)
            .map_err(|error| format!("Cannot reload buffer: {error:?}"))?;
        let Some(path) = self.disk.path(number).map(Path::to_path_buf) else {
            return Err("Buffer has no disk file".to_string());
        };
        let fingerprint = Fingerprint::read(&path).map_err(|error| error.to_string())?;
        self.disk.accept(number, fingerprint);
        self.reveal_cursor();
        Ok(())
    }

    fn buffer_name(&self, number: usize) -> String {
        self.editor
            .buffer_name(number)
            .unwrap_or("[No Name]")
            .to_string()
    }

    pub(super) fn open_files(&self) -> Vec<(usize, PathBuf)> {
        self.editor
            .all_buffer_numbers()
            .into_iter()
            .filter(|number| !self.editor.buffer_is_readonly(*number).unwrap_or(false))
            .filter(|number| !self.directory.contains(*number))
            .filter_map(|number| {
                let path = self.editor.buffer_path(number)?;
                Some((number, absolute_path(Path::new(path))))
            })
            .collect()
    }

    pub(super) fn reload_open_files(&mut self) {
        self.sync_watched_files();
        let paths = self
            .open_files()
            .into_iter()
            .map(|(_, path)| path)
            .collect::<Vec<_>>();
        self.handle_disk_paths(&paths);
    }
}

fn absolute_path(path: &Path) -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_default();
    paths::absolute(&path.to_string_lossy(), &cwd)
}

#[cfg(test)]
mod tests;
