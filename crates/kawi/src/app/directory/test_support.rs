use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use super::fs::DirectoryFs;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Call {
    Rename(PathBuf, PathBuf),
    Copy(PathBuf, PathBuf),
    Trash(PathBuf),
    CreateFile(PathBuf),
    CreateDirectory(PathBuf),
}

#[derive(Default)]
pub(super) struct FakeFs {
    paths: BTreeMap<PathBuf, bool>,
    pub(super) calls: Vec<Call>,
    fail_at: Option<usize>,
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
        }
    }

    pub(super) fn fail_at(mut self, index: usize) -> Self {
        self.fail_at = Some(index);
        self
    }

    fn record(&mut self, call: Call) -> io::Result<()> {
        if self.fail_at == Some(self.calls.len()) {
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

    fn trash(&mut self, path: &Path) -> io::Result<()> {
        self.record(Call::Trash(path.to_path_buf()))?;
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
}
