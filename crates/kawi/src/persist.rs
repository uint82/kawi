use anyhow::{Context, Result};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

pub(crate) fn write_atomically(path: &Path, text: &str) -> Result<()> {
    write_atomically_bytes(path, text.as_bytes())
}

pub(crate) fn write_atomically_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    write_atomically_with_mode(path, bytes, None)
}

pub(crate) fn write_private_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
    write_atomically_with_mode(path, bytes, Some(0o600))
}

fn write_atomically_with_mode(path: &Path, bytes: &[u8], mode: Option<u32>) -> Result<()> {
    let parent = path.parent().context("state file has no parent")?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    let temp = temp_path(path);
    let result = write_temp_file(&temp, bytes, mode)
        .and_then(|()| {
            fs::rename(&temp, path).with_context(|| format!("failed to replace {}", path.display()))
        })
        .and_then(|()| sync_directory(parent));
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn write_temp_file(path: &Path, bytes: &[u8], mode: Option<u32>) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    apply_mode(&mut options, mode);
    let mut file = options
        .open(path)
        .with_context(|| format!("failed to write {}", path.display()))?;
    apply_permissions(&file, mode)?;
    file.write_all(bytes)
        .with_context(|| format!("failed to write {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("failed to sync {}", path.display()))
}

#[cfg(unix)]
fn apply_mode(options: &mut OpenOptions, mode: Option<u32>) {
    if let Some(mode) = mode {
        options.mode(mode);
    }
}

#[cfg(not(unix))]
fn apply_mode(_options: &mut OpenOptions, _mode: Option<u32>) {}

#[cfg(unix)]
fn apply_permissions(file: &File, mode: Option<u32>) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if let Some(mode) = mode {
        file.set_permissions(fs::Permissions::from_mode(mode))
            .context("failed to set private permissions")?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn apply_permissions(_file: &File, _mode: Option<u32>) -> Result<()> {
    Ok(())
}

static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

fn temp_path(path: &Path) -> PathBuf {
    let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    path.with_extension(format!("{}.{id}.tmp", std::process::id()))
}

#[cfg(unix)]
pub(crate) fn sync_directory(path: &Path) -> Result<()> {
    File::open(path)
        .with_context(|| format!("failed to open {}", path.display()))?
        .sync_all()
        .with_context(|| format!("failed to sync {}", path.display()))
}

#[cfg(not(unix))]
pub(crate) fn sync_directory(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kawi_persist_{}_{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn replaces_file_contents() {
        let dir = temp_dir("replace");
        let path = dir.join("state");
        write_atomically(&path, "first").unwrap();
        write_atomically(&path, "second").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "second");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn creates_parent_directories() {
        let dir = temp_dir("parents");
        let path = dir.join("nested").join("state");
        write_atomically(&path, "text").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "text");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn writes_private_bytes() {
        let dir = temp_dir("private");
        let path = dir.join("snapshot");
        write_private_atomically(&path, b"text").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"text");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
        fs::remove_dir_all(dir).unwrap();
    }
}
