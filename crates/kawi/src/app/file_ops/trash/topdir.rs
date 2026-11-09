use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(super) fn mount_point(path: &Path) -> Option<PathBuf> {
    let start = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => path,
        _ => path.parent()?,
    };
    let device = device_of(start)?;
    let mut top = start;
    for ancestor in start.ancestors() {
        if device_of(ancestor) != Some(device) {
            break;
        }
        top = ancestor;
    }
    Some(top.to_path_buf())
}

pub(super) fn user_id() -> Option<String> {
    #[cfg(unix)]
    {
        Some(unsafe { libc::geteuid() }.to_string())
    }
    #[cfg(not(unix))]
    {
        std::env::var("USERNAME").ok()
    }
}

#[cfg(any(target_os = "macos", test))]
pub(super) fn is_directory(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
}

pub(super) fn ensure_directory(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("trash path is not a directory: {}", path.display()),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir(path),
        Err(error) => Err(error),
    }
}

pub(super) fn ensure_root(path: &Path) -> io::Result<bool> {
    match ensure_directory(path) {
        Ok(()) => {
            verify_owner(path)?;
            Ok(false)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::create_dir(path)?;
            verify_owner(path)?;
            Ok(true)
        }
        Err(error) => Err(error),
    }
}

fn verify_owner(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = fs::symlink_metadata(path)?;
        let owner = metadata.uid();
        let current = unsafe { libc::geteuid() };
        if owner != current {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("trash path is owned by uid {owner}: {}", path.display()),
            ));
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

fn device_of(path: &Path) -> Option<u64> {
    for ancestor in path.ancestors() {
        if let Ok(metadata) = fs::metadata(ancestor) {
            return Some(device(&metadata));
        }
    }
    None
}

pub(super) fn same_device(left: &Path, right: &Path) -> bool {
    match (fs::metadata(left), device_of(right)) {
        (Ok(left), Some(right)) => device(&left) == right,
        _ => false,
    }
}

#[cfg(unix)]
fn device(metadata: &fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    metadata.dev()
}

#[cfg(not(unix))]
fn device(_metadata: &fs::Metadata) -> u64 {
    0
}
