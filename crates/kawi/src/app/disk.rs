use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use kawi_editor::paths;
use kawi_watch::{Fingerprint, WatchService};

use super::App;

const STATUS_CHANGED: &str = "changed";
const STATUS_DELETED: &str = "deleted";

#[derive(Default)]
pub(super) struct DiskState {
    files: BTreeMap<usize, DiskFile>,
    watched: BTreeSet<PathBuf>,
}

struct DiskFile {
    path: PathBuf,
    base: Option<Fingerprint>,
    current: Option<Fingerprint>,
    changed: bool,
    deleted: bool,
    conflict: bool,
    dismissed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Observation {
    Ignored,
    Same,
    Changed,
    Deleted,
}

impl DiskState {
    fn sync(&mut self, files: &[(usize, PathBuf)]) -> Option<BTreeSet<PathBuf>> {
        let numbers: BTreeSet<usize> = files.iter().map(|(number, _)| *number).collect();
        self.files.retain(|number, _| numbers.contains(number));
        for (number, path) in files {
            match self.files.get_mut(number) {
                Some(file) if file.path == *path => {}
                Some(file) => *file = DiskFile::new(path.clone()),
                None => {
                    self.files.insert(*number, DiskFile::new(path.clone()));
                }
            }
        }
        let parents: BTreeSet<PathBuf> = files
            .iter()
            .filter_map(|(_, path)| path.parent().map(Path::to_path_buf))
            .collect();
        if parents == self.watched {
            return None;
        }
        self.watched = parents.clone();
        Some(parents)
    }

    pub(super) fn path(&self, number: usize) -> Option<&Path> {
        self.files.get(&number).map(|file| file.path.as_path())
    }

    fn status(&self, number: usize) -> Option<&'static str> {
        let file = self.files.get(&number)?;
        if file.deleted {
            Some(STATUS_DELETED)
        } else if file.changed {
            Some(STATUS_CHANGED)
        } else {
            None
        }
    }

    pub(super) fn conflict(&self, number: usize) -> bool {
        self.files.get(&number).is_some_and(|file| file.conflict)
    }

    fn any_conflict(&self) -> bool {
        self.files.values().any(|file| file.conflict)
    }

    fn needs_prompt(&self, number: usize) -> bool {
        self.files
            .get(&number)
            .is_some_and(|file| file.conflict && !file.dismissed)
    }

    fn open_prompt(&mut self, number: usize) {
        if let Some(file) = self.files.get_mut(&number) {
            file.dismissed = false;
        }
    }

    fn dismiss(&mut self, number: usize) {
        if let Some(file) = self.files.get_mut(&number) {
            file.dismissed = true;
        }
    }

    fn matching_buffers(&self, paths: &[PathBuf]) -> Vec<usize> {
        self.files
            .iter()
            .filter(|(_, file)| paths.iter().any(|path| paths::same_file(path, &file.path)))
            .map(|(number, _)| *number)
            .collect()
    }

    fn observe(&mut self, number: usize, observed: Option<Fingerprint>) -> Observation {
        let Some(file) = self.files.get_mut(&number) else {
            return Observation::Ignored;
        };
        file.observe(observed)
    }

    fn accept(&mut self, number: usize, fingerprint: Fingerprint) {
        if let Some(file) = self.files.get_mut(&number) {
            file.accept(fingerprint);
        }
    }

    fn accept_current(&mut self, number: usize) {
        if let Some(file) = self.files.get_mut(&number) {
            file.accept_current();
        }
    }

    fn mark_changed(&mut self, number: usize) {
        if let Some(file) = self.files.get_mut(&number) {
            file.mark_changed();
        }
    }

    fn mark_conflict(&mut self, number: usize) {
        if let Some(file) = self.files.get_mut(&number) {
            file.mark_conflict();
        }
    }

    fn record_saved(&mut self, number: usize, path: PathBuf, fingerprint: Option<Fingerprint>) {
        match self.files.get_mut(&number) {
            Some(file) if file.path == path => file.record_saved(fingerprint),
            Some(file) => {
                *file = DiskFile::from_fingerprint(path, fingerprint);
            }
            None => {
                self.files
                    .insert(number, DiskFile::from_fingerprint(path, fingerprint));
            }
        }
    }
}

impl DiskFile {
    fn new(path: PathBuf) -> Self {
        let fingerprint = Fingerprint::read(&path).ok();
        Self::from_fingerprint(path, fingerprint)
    }

    fn from_fingerprint(path: PathBuf, fingerprint: Option<Fingerprint>) -> Self {
        Self {
            path,
            base: fingerprint,
            current: fingerprint,
            changed: false,
            deleted: false,
            conflict: false,
            dismissed: false,
        }
    }

    fn observe(&mut self, observed: Option<Fingerprint>) -> Observation {
        let Some(observed) = observed else {
            if self.base.is_none() && self.current.is_none() {
                return Observation::Ignored;
            }
            if self.deleted {
                return Observation::Ignored;
            }
            self.current = None;
            self.changed = true;
            self.deleted = true;
            self.conflict = false;
            self.dismissed = false;
            return Observation::Deleted;
        };
        if self
            .base
            .as_ref()
            .is_some_and(|base| base.matches(&observed))
        {
            self.accept(observed);
            return Observation::Same;
        }
        if !self.deleted
            && self
                .current
                .as_ref()
                .is_some_and(|current| current.matches(&observed))
        {
            return Observation::Ignored;
        }
        self.current = Some(observed);
        self.deleted = false;
        Observation::Changed
    }

    fn accept(&mut self, fingerprint: Fingerprint) {
        self.base = Some(fingerprint);
        self.current = Some(fingerprint);
        self.changed = false;
        self.deleted = false;
        self.conflict = false;
        self.dismissed = false;
    }

    fn accept_current(&mut self) {
        self.base = self.current;
        self.changed = false;
        self.deleted = false;
        self.conflict = false;
        self.dismissed = false;
    }

    fn mark_changed(&mut self) {
        self.changed = true;
        self.conflict = false;
        self.dismissed = false;
    }

    fn mark_conflict(&mut self) {
        self.changed = true;
        self.conflict = true;
        self.dismissed = false;
    }

    fn record_saved(&mut self, fingerprint: Option<Fingerprint>) {
        self.base = fingerprint;
        self.current = fingerprint;
        self.changed = false;
        self.deleted = false;
        self.conflict = false;
        self.dismissed = false;
    }
}

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
        for number in self.disk.matching_buffers(paths) {
            let observed = self
                .disk
                .path(number)
                .and_then(|path| Fingerprint::read(path).ok());
            match self.disk.observe(number, observed) {
                Observation::Ignored => {}
                Observation::Same => {}
                Observation::Deleted => self.report_deleted_file(number),
                Observation::Changed => self.handle_changed_file(number, observed),
            }
            self.close_resolved_conflict(number);
        }
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
    }

    pub(super) fn record_all_saved(&mut self) {
        for (number, path) in self.open_files() {
            let fingerprint = Fingerprint::read(&path).ok();
            self.disk.record_saved(number, path, fingerprint);
        }
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
        self.disk.status(number)
    }

    pub(super) fn disk_conflict(&self, number: usize) -> bool {
        self.disk.conflict(number)
    }

    pub(super) fn disk_deleted(&self, number: usize) -> bool {
        self.disk.status(number) == Some(STATUS_DELETED)
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

    fn open_files(&self) -> Vec<(usize, PathBuf)> {
        self.editor
            .all_buffer_numbers()
            .into_iter()
            .filter_map(|number| {
                let path = self.editor.buffer_path(number)?;
                Some((number, absolute_path(Path::new(path))))
            })
            .collect()
    }
}

fn absolute_path(path: &Path) -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_default();
    paths::absolute(&path.to_string_lossy(), &cwd)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use kawi_buffer::Buffer;
    use kawi_editor::Action;

    use super::*;
    use crate::keymap::Flavor;

    fn temp_dir(name: &str) -> PathBuf {
        crate::test_support::temp_dir(name)
    }

    fn app_with(dir: &Path, name: &str) -> App {
        let path = dir.join(name);
        let mut app = App::new(
            Buffer::from_file(path.to_str().expect("path")).expect("buffer"),
            Flavor::Vim,
        );
        app.sync_watched_files();
        app
    }

    fn edit(app: &mut App, text: &str) {
        app.editor.apply(Action::EnterInsert);
        for ch in text.chars() {
            app.editor.apply(Action::InsertChar(ch));
        }
        app.editor.apply(Action::ExitInsert);
    }

    fn message(app: &App) -> String {
        app.last_message()
            .map(str::to_string)
            .or_else(|| app.pager_lines().map(|lines| lines.join(" ")))
            .unwrap_or_default()
    }

    #[test]
    fn reloads_clean_buffers_and_preserves_the_cursor() {
        let dir = temp_dir("reload");
        let path = dir.join("a.txt");
        fs::write(&path, "one\ntwo\nthree\n").expect("write");
        let mut app = app_with(&dir, "a.txt");
        app.editor.apply(Action::JumpTo(2, 2));
        fs::write(&path, "changed\n").expect("external");
        app.handle_disk_paths(&[path]);
        assert_eq!(app.editor.visible_lines(0, 1), vec!["changed"]);
        assert_eq!(app.cursor_position(), (0, 2));
        assert!(app.disk_status(1).is_none());
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn identical_content_does_not_reload() {
        let dir = temp_dir("same");
        let path = dir.join("a.txt");
        fs::write(&path, "alpha\n").expect("write");
        let mut app = app_with(&dir, "a.txt");
        let revision = app.editor.buffer_revision();
        fs::write(&path, "alpha\n").expect("rewrite");
        app.handle_disk_paths(&[path]);
        assert_eq!(app.editor.buffer_revision(), revision);
        assert!(app.disk_status(1).is_none());
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn dirty_buffers_become_conflicts_without_overwriting() {
        let dir = temp_dir("dirty");
        let path = dir.join("a.txt");
        fs::write(&path, "alpha\n").expect("write");
        let mut app = app_with(&dir, "a.txt");
        edit(&mut app, "x");
        fs::write(&path, "theirs\n").expect("external");
        app.handle_disk_paths(&[path]);
        assert_eq!(app.editor.visible_lines(0, 1), vec!["xalpha"]);
        assert!(app.editor.is_dirty());
        assert_eq!(app.disk_status(1), Some(STATUS_CHANGED));
        assert!(app.disk_conflict(1));
        assert!(message(&app).ends_with("a.txt"));
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn keep_mine_accepts_current_disk_state() {
        let dir = temp_dir("keep");
        let path = dir.join("a.txt");
        fs::write(&path, "alpha\n").expect("write");
        let mut app = app_with(&dir, "a.txt");
        edit(&mut app, "x");
        fs::write(&path, "theirs\n").expect("external");
        app.handle_disk_paths(std::slice::from_ref(&path));
        app.keep_buffer_version(1);
        assert!(!app.disk_conflict(1));
        assert!(app.disk_status(1).is_none());
        assert_eq!(app.editor.visible_lines(0, 1), vec!["xalpha"]);
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn load_theirs_replaces_dirty_content() {
        let dir = temp_dir("theirs");
        let path = dir.join("a.txt");
        fs::write(&path, "alpha\n").expect("write");
        let mut app = app_with(&dir, "a.txt");
        edit(&mut app, "x");
        fs::write(&path, "theirs\n").expect("external");
        app.handle_disk_paths(std::slice::from_ref(&path));
        app.load_disk_version(1).expect("load");
        assert_eq!(app.editor.visible_lines(0, 1), vec!["theirs"]);
        assert!(!app.editor.is_dirty());
        assert!(!app.disk_conflict(1));
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn deletion_keeps_the_buffer_and_sets_a_flag() {
        let dir = temp_dir("deleted");
        let path = dir.join("a.txt");
        fs::write(&path, "alpha\n").expect("write");
        let mut app = app_with(&dir, "a.txt");
        fs::remove_file(&path).expect("remove");
        app.handle_disk_paths(&[path]);
        assert_eq!(app.editor.visible_lines(0, 1), vec!["alpha"]);
        assert_eq!(app.disk_status(1), Some(STATUS_DELETED));
        assert!(message(&app).contains("Deleted on disk"));
        assert!(message(&app).contains("kept"));
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn saving_records_the_new_disk_fingerprint() {
        let dir = temp_dir("save");
        let path = dir.join("a.txt");
        fs::write(&path, "alpha\n").expect("write");
        let mut app = app_with(&dir, "a.txt");
        edit(&mut app, "x");
        app.handle_save(false).expect("save");
        app.handle_disk_paths(std::slice::from_ref(&path));
        assert!(app.disk_status(1).is_none());
        assert_eq!(fs::read_to_string(path).expect("read"), "xalpha\n");
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn recreated_identical_files_clear_deleted_flags() {
        let dir = temp_dir("restored");
        let path = dir.join("a.txt");
        fs::write(&path, "alpha\n").expect("write");
        let mut app = app_with(&dir, "a.txt");
        fs::remove_file(&path).expect("remove");
        app.handle_disk_paths(std::slice::from_ref(&path));
        fs::write(&path, "alpha\n").expect("recreate");
        app.handle_disk_paths(&[path]);
        assert!(app.disk_status(1).is_none());
        assert_eq!(app.editor.visible_lines(0, 1), vec!["alpha"]);
        fs::remove_dir_all(dir).expect("cleanup");
    }
}
