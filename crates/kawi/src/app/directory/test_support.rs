use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::fs::DirectoryFs;
use crate::app::App;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Call {
    Rename(PathBuf, PathBuf),
    Copy(PathBuf, PathBuf),
    Trash(PathBuf),
    Remove(PathBuf),
    CreateFile(PathBuf),
    CreateDirectory(PathBuf),
}

#[derive(Default)]
pub(super) struct FakeFs {
    paths: BTreeMap<PathBuf, bool>,
    pub(super) calls: Vec<Call>,
    fail_at: Option<usize>,
    denied_write: BTreeSet<PathBuf>,
    denied_directory_write: BTreeSet<PathBuf>,
    denied_read: BTreeSet<PathBuf>,
    denied_entry_remove: BTreeSet<PathBuf>,
    denied_delete: BTreeSet<PathBuf>,
}

impl FakeFs {
    pub(super) fn new(paths: &[(&str, bool)]) -> Self {
        Self {
            paths: paths
                .iter()
                .map(|(path, is_dir)| (PathBuf::from(path), *is_dir))
                .collect(),
            calls: Vec::new(),
            fail_at: None,
            denied_write: BTreeSet::new(),
            denied_directory_write: BTreeSet::new(),
            denied_read: BTreeSet::new(),
            denied_entry_remove: BTreeSet::new(),
            denied_delete: BTreeSet::new(),
        }
    }

    pub(super) fn fail_at(mut self, index: usize) -> Self {
        self.fail_at = Some(index);
        self
    }

    pub(super) fn deny_write(mut self, path: &str) -> Self {
        self.denied_write.insert(PathBuf::from(path));
        self
    }

    pub(super) fn deny_directory_write(mut self, path: &str) -> Self {
        self.denied_directory_write.insert(PathBuf::from(path));
        self
    }

    pub(super) fn deny_read(mut self, path: &str) -> Self {
        self.denied_read.insert(PathBuf::from(path));
        self
    }

    pub(super) fn deny_entry_remove(mut self, path: &str) -> Self {
        self.denied_entry_remove.insert(PathBuf::from(path));
        self
    }

    pub(super) fn deny_delete(mut self, path: &str) -> Self {
        self.denied_delete.insert(PathBuf::from(path));
        self
    }

    fn record(&mut self, call: Call) -> io::Result<()> {
        if self.fail_at == Some(self.calls.len()) {
            self.fail_at = None;
            return Err(io::Error::other("injected failure"));
        }
        self.calls.push(call);
        Ok(())
    }
}

impl DirectoryFs for FakeFs {
    fn rename(&mut self, from: &Path, to: &Path) -> io::Result<()> {
        self.record(Call::Rename(from.to_path_buf(), to.to_path_buf()))?;
        let is_dir = self.paths.remove(from).unwrap_or(false);
        self.paths.insert(to.to_path_buf(), is_dir);
        Ok(())
    }

    fn copy(&mut self, from: &Path, to: &Path) -> io::Result<()> {
        self.record(Call::Copy(from.to_path_buf(), to.to_path_buf()))?;
        let is_dir = self.paths.get(from).copied().unwrap_or(false);
        self.paths.insert(to.to_path_buf(), is_dir);
        Ok(())
    }

    fn finish_delete(&mut self, staged: &Path, original: &Path) -> io::Result<()> {
        self.record(Call::Trash(original.to_path_buf()))?;
        self.paths.remove(staged);
        Ok(())
    }

    fn remove_file(&mut self, path: &Path) -> io::Result<()> {
        self.record(Call::Remove(path.to_path_buf()))?;
        self.paths.remove(path);
        Ok(())
    }

    fn remove_directory(&mut self, path: &Path) -> io::Result<()> {
        self.record(Call::Remove(path.to_path_buf()))?;
        self.paths.remove(path);
        Ok(())
    }

    fn remove_tree(&mut self, path: &Path) -> io::Result<()> {
        self.record(Call::Remove(path.to_path_buf()))?;
        self.paths.remove(path);
        Ok(())
    }

    fn create_file(&mut self, path: &Path) -> io::Result<()> {
        self.record(Call::CreateFile(path.to_path_buf()))?;
        self.paths.insert(path.to_path_buf(), false);
        Ok(())
    }

    fn create_dir(&mut self, path: &Path) -> io::Result<()> {
        self.record(Call::CreateDirectory(path.to_path_buf()))?;
        self.paths.insert(path.to_path_buf(), true);
        Ok(())
    }

    fn exists(&self, path: &Path) -> bool {
        self.paths.contains_key(path)
    }

    fn is_dir(&self, path: &Path) -> bool {
        self.paths.get(path).copied().unwrap_or(false)
    }

    fn can_modify_directory(&self, path: &Path) -> bool {
        !self.denied_write.contains(path)
    }

    fn can_write_directory(&self, path: &Path) -> bool {
        !self.denied_directory_write.contains(path)
    }

    fn can_read(&self, path: &Path) -> bool {
        !self.denied_read.contains(path)
    }

    fn can_remove_entry(&self, path: &Path) -> bool {
        !self.denied_entry_remove.contains(path)
    }

    fn delete_error(&self, path: &Path) -> Option<String> {
        self.denied_delete
            .contains(path)
            .then(|| "no trash available on this filesystem".to_string())
    }
}

pub(super) fn message_text(app: &App) -> String {
    app.last_message()
        .map(str::to_string)
        .or_else(|| app.pager_lines().map(|lines| lines.concat()))
        .unwrap_or_default()
}

pub(super) fn settle(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.directory_loading.has_loads() || app.has_pending_permission_check() {
        app.poll_directory_loads();
        app.poll_permission_check();
        if app.directory_loading.has_loads() || app.has_pending_permission_check() {
            assert!(Instant::now() < deadline, "directory load timed out");
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
