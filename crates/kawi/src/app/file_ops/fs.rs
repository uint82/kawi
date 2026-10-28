use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

mod access;
mod copy;

use copy::copy_across_filesystems;

pub(in crate::app) use access::{can_modify_directory, can_read, can_remove_tree};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum RenameKind {
    Moved,
    Copied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) struct RenameOutcome {
    pub kind: RenameKind,
    pub symlink: bool,
}

pub(in crate::app) fn copy_path(old: &Path, new: &Path, force: bool) -> io::Result<()> {
    copy_across_filesystems(old, new, force)
}

pub(in crate::app) fn rename_path(
    old: &Path,
    new: &Path,
    force: bool,
) -> io::Result<RenameOutcome> {
    let source = fs::symlink_metadata(old)?;
    let symlink = source.file_type().is_symlink();
    let target = fs::symlink_metadata(new);
    let case_only = target
        .as_ref()
        .is_ok_and(|target| same_entry(&source, target) && is_case_variant(old, new));
    let target_is_directory = target
        .as_ref()
        .is_ok_and(|target| !case_only && target.is_dir() && !target.file_type().is_symlink());
    if target_is_directory || (!force && target.is_ok() && !case_only) {
        return Err(already_exists(new));
    }
    let result = if force || case_only {
        fs::rename(old, new)
    } else {
        rename_noreplace(old, new)
    };
    match result {
        Ok(()) => Ok(RenameOutcome {
            kind: RenameKind::Moved,
            symlink,
        }),
        Err(error) if error.kind() == io::ErrorKind::CrossesDevices => {
            copy_across_filesystems(old, new, force)?;
            remove_path(old, true)?;
            Ok(RenameOutcome {
                kind: RenameKind::Copied,
                symlink,
            })
        }
        Err(error) => Err(error),
    }
}

pub(in crate::app) fn remove_path(path: &Path, permanent: bool) -> io::Result<()> {
    if !permanent {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "permanent removal was not requested",
        ));
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

pub(super) fn count_tree(path: &Path) -> io::Result<usize> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Ok(0);
    }
    let mut count: usize = 0;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        count = count.saturating_add(1);
        let metadata = fs::symlink_metadata(entry.path())?;
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            count = count.saturating_add(count_tree(&entry.path())?);
        }
    }
    Ok(count)
}

pub(in crate::app) fn path_exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn same_entry(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        left.dev() == right.dev() && left.ino() == right.ino()
    }
    #[cfg(not(unix))]
    {
        let _ = (left, right);
        false
    }
}

fn is_case_variant(old: &Path, new: &Path) -> bool {
    let (Some(old_name), Some(new_name)) = (old.file_name(), new.file_name()) else {
        return false;
    };
    old_name != new_name
        && old_name
            .to_string_lossy()
            .to_lowercase()
            .eq(&new_name.to_string_lossy().to_lowercase())
}

fn already_exists(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("{} already exists", path.display()),
    )
}

pub(in crate::app) fn create_empty_file(path: &Path) -> io::Result<()> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map(|_| ())
}

pub(in crate::app) fn create_directory(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)
}

pub(super) fn missing_parent(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    (!parent.as_os_str().is_empty() && !parent.is_dir()).then(|| parent.to_path_buf())
}

fn rename_noreplace(old: &Path, new: &Path) -> io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        match linux_rename_noreplace(old, new) {
            Ok(()) => return Ok(()),
            Err(error)
                if matches!(
                    error.raw_os_error(),
                    Some(libc::ENOSYS) | Some(libc::EINVAL) | Some(libc::EOPNOTSUPP)
                ) => {}
            Err(error) => return Err(error),
        }
    }
    if fs::symlink_metadata(new).is_ok() {
        return Err(already_exists(new));
    }
    let metadata = fs::symlink_metadata(old)?;
    if metadata.is_dir() || metadata.file_type().is_symlink() {
        return fs::rename(old, new);
    }
    fs::hard_link(old, new)?;
    fs::remove_file(old)
}

#[cfg(target_os = "linux")]
fn linux_rename_noreplace(old: &Path, new: &Path) -> io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let old = CString::new(old.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains a NUL byte"))?;
    let new = CString::new(new.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains a NUL byte"))?;
    let result = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            libc::AT_FDCWD,
            old.as_ptr(),
            libc::AT_FDCWD,
            new.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(test)]
mod tests;
