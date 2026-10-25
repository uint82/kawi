use std::fs::{self, File, FileTimes, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::{remove_path, rename_noreplace};

static NEXT_STAGED: AtomicU64 = AtomicU64::new(1);

pub(super) fn copy_across_filesystems(old: &Path, new: &Path, force: bool) -> io::Result<()> {
    let staged = staged_path(new);
    copy_tree(old, &staged)?;
    if let Err(error) = promote_copy(&staged, new, force) {
        let _ = remove_path(&staged, true);
        return Err(error);
    }
    Ok(())
}

fn staged_path(new: &Path) -> PathBuf {
    let name = new
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "item".to_string());
    let staged = format!(
        ".{name}.kawi-copy-{}-{:016x}",
        std::process::id(),
        staged_suffix()
    );
    new.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from(&staged), |parent| parent.join(&staged))
}

fn staged_suffix() -> u64 {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let mut bytes = [0_u8; 8];
        let read = unsafe { libc::getrandom(bytes.as_mut_ptr().cast(), bytes.len(), 0) };
        if read == bytes.len() as isize {
            return u64::from_ne_bytes(bytes);
        }
    }
    let counter = NEXT_STAGED.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);
    counter ^ nanos.rotate_left(17) ^ u64::from(std::process::id())
}

pub(super) fn promote_copy(staged: &Path, new: &Path, force: bool) -> io::Result<()> {
    if !force {
        return rename_noreplace(staged, new);
    }
    fs::rename(staged, new)
}

pub(super) fn copy_tree(old: &Path, new: &Path) -> io::Result<()> {
    copy_entry(old, new).inspect_err(|_| {
        let _ = remove_path(new, true);
    })
}

fn copy_entry(old: &Path, new: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(old)?;
    if metadata.file_type().is_symlink() {
        copy_symlink(old, new)?;
        return preserve_symlink_times(&metadata, new);
    }
    if metadata.is_dir() {
        fs::create_dir(new)?;
        for entry in fs::read_dir(old)? {
            let entry = entry?;
            copy_entry(&entry.path(), &new.join(entry.file_name()))?;
        }
        File::open(new)?.sync_all()?;
    } else {
        copy_file(old, new)?;
    }
    preserve_metadata(&metadata, new, metadata.is_dir())
}

fn copy_file(old: &Path, new: &Path) -> io::Result<()> {
    let mut source = File::open(old)?;
    let mut target = OpenOptions::new().write(true).create_new(true).open(new)?;
    io::copy(&mut source, &mut target)?;
    target.sync_all()
}

fn preserve_metadata(metadata: &fs::Metadata, path: &Path, is_directory: bool) -> io::Result<()> {
    if !is_directory {
        fs::set_permissions(path, metadata.permissions())?;
    }
    let mut times = FileTimes::new();
    let mut specified = false;
    if let Ok(accessed) = metadata.accessed() {
        times = times.set_accessed(accessed);
        specified = true;
    }
    if let Ok(modified) = metadata.modified() {
        times = times.set_modified(modified);
        specified = true;
    }
    if specified {
        File::open(path)?.set_times(times)?;
    }
    if is_directory {
        fs::set_permissions(path, metadata.permissions())?;
    }
    Ok(())
}

#[cfg(unix)]
fn copy_symlink(old: &Path, new: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(fs::read_link(old)?, new)
}

#[cfg(not(unix))]
fn copy_symlink(old: &Path, new: &Path) -> io::Result<()> {
    fs::copy(old, new).map(|_| ())
}

fn preserve_symlink_times(metadata: &fs::Metadata, path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        use std::time::UNIX_EPOCH;

        let (Ok(accessed), Ok(modified)) = (metadata.accessed(), metadata.modified()) else {
            return Ok(());
        };
        let times = [
            to_timespec(accessed, UNIX_EPOCH),
            to_timespec(modified, UNIX_EPOCH),
        ];
        let path = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains a NUL byte"))?;
        let result = unsafe {
            libc::utimensat(
                libc::AT_FDCWD,
                path.as_ptr(),
                times.as_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (metadata, path);
        Ok(())
    }
}

#[cfg(unix)]
fn to_timespec(time: std::time::SystemTime, epoch: std::time::SystemTime) -> libc::timespec {
    match time.duration_since(epoch) {
        Ok(duration) => libc::timespec {
            tv_sec: duration.as_secs() as libc::time_t,
            tv_nsec: duration.subsec_nanos() as libc::c_long,
        },
        Err(error) => {
            let duration = error.duration();
            let mut seconds = -(duration.as_secs() as i64);
            let mut nanos = duration.subsec_nanos() as i64;
            if nanos > 0 {
                seconds -= 1;
                nanos = 1_000_000_000 - nanos;
            }
            libc::timespec {
                tv_sec: seconds as libc::time_t,
                tv_nsec: nanos as libc::c_long,
            }
        }
    }
}
