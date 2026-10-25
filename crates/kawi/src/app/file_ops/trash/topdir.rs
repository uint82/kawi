use super::*;

pub(super) fn topdir_trash(target: &Path) -> Option<TrashDir> {
    let topdir = mount_point(target)?;
    let shared = topdir.join(TOP_TRASH);
    if is_secure_shared_trash(&shared) {
        return Some(TrashDir {
            root: shared.join(user_id()?),
            topdir: Some(topdir),
        });
    }
    Some(TrashDir {
        root: topdir.join(format!(".Trash-{}", user_id()?)),
        topdir: Some(topdir),
    })
}

fn mount_point(path: &Path) -> Option<PathBuf> {
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

fn is_secure_shared_trash(path: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o1000 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
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

pub(super) fn ensure_root(path: &Path) -> io::Result<()> {
    match ensure_directory(path) {
        Ok(()) => verify_owner(path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::create_dir(path)?;
            verify_owner(path)
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
