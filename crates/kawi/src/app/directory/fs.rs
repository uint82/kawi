use std::io;
use std::path::Path;

use super::super::file_ops::fs as file_fs;
use super::super::file_ops::trash::{TrashDir, TrashError};

pub(super) trait DirectoryFs {
    fn rename(&mut self, from: &Path, to: &Path) -> io::Result<()>;
    fn try_rename(&mut self, from: &Path, to: &Path) -> io::Result<()>;
    fn stage_copy(&mut self, from: &Path, stage: &Path) -> io::Result<()>;
    fn promote_stage(&mut self, stage: &Path, to: &Path) -> io::Result<()>;
    fn finish_delete(&mut self, staged: &Path, original: &Path) -> io::Result<()>;
    fn remove_file(&mut self, path: &Path) -> io::Result<()>;
    fn remove_directory(&mut self, path: &Path) -> io::Result<()>;
    fn remove_tree(&mut self, path: &Path) -> io::Result<()>;
    fn sync_parent(&self, path: &Path) -> io::Result<()>;
    fn create_file(&mut self, path: &Path) -> io::Result<()>;
    fn create_dir(&mut self, path: &Path) -> io::Result<()>;
    fn exists(&self, path: &Path) -> bool;
    fn is_dir(&self, path: &Path) -> bool;
    fn can_modify_directory(&self, path: &Path) -> bool;
    fn can_write_directory(&self, path: &Path) -> bool;
    fn can_read(&self, path: &Path) -> bool;
    fn can_remove_entry(&self, path: &Path) -> bool;
    fn delete_error(&self, path: &Path) -> Option<String>;
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

    fn try_rename(&mut self, from: &Path, to: &Path) -> io::Result<()> {
        file_fs::try_rename_path(from, to, false).map(|_| ())
    }

    fn stage_copy(&mut self, from: &Path, stage: &Path) -> io::Result<()> {
        file_fs::stage_copy(from, stage)
    }

    fn promote_stage(&mut self, stage: &Path, to: &Path) -> io::Result<()> {
        file_fs::promote_stage(stage, to)
    }

    fn finish_delete(&mut self, staged: &Path, original: &Path) -> io::Result<()> {
        if self.permanent_delete {
            return file_fs::remove_path(staged, true);
        }
        let trash = TrashDir::for_target(original)
            .ok_or_else(|| io::Error::other("trash is unavailable"))?;
        trash
            .trash_as(staged, original)
            .map(|_| ())
            .map_err(|error| match error {
                TrashError::Io(error) => error,
                TrashError::Unavailable(message) => io::Error::other(message),
            })
    }

    fn remove_file(&mut self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(path)
    }

    fn remove_directory(&mut self, path: &Path) -> io::Result<()> {
        std::fs::remove_dir(path)
    }

    fn remove_tree(&mut self, path: &Path) -> io::Result<()> {
        file_fs::remove_path(path, true)
    }

    fn sync_parent(&self, path: &Path) -> io::Result<()> {
        file_fs::sync_parent(path)
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

    fn can_write_directory(&self, path: &Path) -> bool {
        file_fs::can_write_directory(path)
    }

    fn can_read(&self, path: &Path) -> bool {
        file_fs::can_read(path)
    }

    fn can_remove_entry(&self, path: &Path) -> bool {
        file_fs::sticky_allows(path)
    }

    fn delete_error(&self, path: &Path) -> Option<String> {
        if self.permanent_delete {
            return (!file_fs::can_remove_tree(path)).then(|| {
                "permission denied; set directory_trash=true to use the trash".to_string()
            });
        }
        let Some(trash) = TrashDir::for_target(path) else {
            return Some(
                "no trash available on this filesystem; set directory_trash=false to delete permanently"
                    .to_string(),
            );
        };
        (!trash.can_trash(path)).then(|| {
            "no writable trash available on this filesystem; set directory_trash=false to delete permanently"
                .to_string()
        })
    }
}
