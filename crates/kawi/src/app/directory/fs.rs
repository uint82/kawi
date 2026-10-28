use std::io;
use std::path::Path;

use super::super::file_ops::fs as file_fs;
use super::super::file_ops::trash::{TrashDir, TrashError};

pub(super) trait DirectoryFs {
    fn rename(&mut self, from: &Path, to: &Path) -> io::Result<()>;
    fn copy(&mut self, from: &Path, to: &Path) -> io::Result<()>;
    fn trash(&mut self, path: &Path) -> io::Result<()>;
    fn create_file(&mut self, path: &Path) -> io::Result<()>;
    fn create_dir(&mut self, path: &Path) -> io::Result<()>;
    fn exists(&self, path: &Path) -> bool;
    fn is_dir(&self, path: &Path) -> bool;
    fn can_modify_directory(&self, path: &Path) -> bool;
    fn can_read(&self, path: &Path) -> bool;
    fn can_delete(&self, path: &Path) -> bool;
}

pub(super) struct RealFs {
    pub(super) permanent_delete: bool,
}

impl RealFs {
    pub(super) fn new(permanent_delete: bool) -> Self {
        Self { permanent_delete }
    }
}

impl DirectoryFs for RealFs {
    fn rename(&mut self, from: &Path, to: &Path) -> io::Result<()> {
        file_fs::rename_path(from, to, false).map(|_| ())
    }

    fn copy(&mut self, from: &Path, to: &Path) -> io::Result<()> {
        file_fs::copy_path(from, to, false)
    }

    fn trash(&mut self, path: &Path) -> io::Result<()> {
        if self.permanent_delete {
            return file_fs::remove_path(path, true);
        }
        let trash =
            TrashDir::for_target(path).ok_or_else(|| io::Error::other("trash is unavailable"))?;
        trash.trash(path).map(|_| ()).map_err(|error| match error {
            TrashError::Io(error) => error,
            TrashError::Unavailable(message) => io::Error::other(message),
        })
    }

    fn create_file(&mut self, path: &Path) -> io::Result<()> {
        file_fs::create_empty_file(path)
    }

    fn create_dir(&mut self, path: &Path) -> io::Result<()> {
        file_fs::create_directory(path)
    }

    fn exists(&self, path: &Path) -> bool {
        file_fs::path_exists(path)
    }

    fn is_dir(&self, path: &Path) -> bool {
        std::fs::symlink_metadata(path)
            .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
    }

    fn can_modify_directory(&self, path: &Path) -> bool {
        file_fs::can_modify_directory(path)
    }

    fn can_read(&self, path: &Path) -> bool {
        file_fs::can_read(path)
    }

    fn can_delete(&self, path: &Path) -> bool {
        if self.permanent_delete {
            return file_fs::can_remove_tree(path);
        }
        TrashDir::for_target(path).is_some_and(|trash| trash.can_trash(path))
    }
}
