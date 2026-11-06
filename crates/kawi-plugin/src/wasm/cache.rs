use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use wasmtime::component::Component;
use wasmtime::Engine;

static CACHE_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

pub(crate) fn set_dir(dir: PathBuf) {
    let _ = CACHE_DIR.set(Some(dir));
}

pub(crate) fn configured_dir() -> Option<&'static Path> {
    CACHE_DIR.get().and_then(|dir| dir.as_deref())
}

pub(crate) fn load(engine: &Engine, dir: &Path, hash: &str) -> Option<Component> {
    let path = component_path(dir, hash);
    let metadata = fs::symlink_metadata(&path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    match unsafe { Component::deserialize_file(engine, &path) } {
        Ok(component) => Some(component),
        Err(_) => {
            let _ = fs::remove_file(&path);
            None
        }
    }
}

pub(crate) fn store(dir: &Path, hash: &str, component: &Component) -> Result<(), String> {
    let bytes = component
        .serialize()
        .map_err(|error| format!("serialize component: {error}"))?;
    prepare_directory(dir)?;
    let target = component_path(dir, hash);
    let temporary = temporary_path(dir, hash);
    let result = write_private(&temporary, &bytes).and_then(|_| fs::rename(&temporary, &target));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|error| format!("store component: {error}"))
}

fn component_path(dir: &Path, hash: &str) -> PathBuf {
    let name = hash.strip_prefix("sha256:").unwrap_or(hash);
    dir.join(format!("{name}.cwasm"))
}

fn temporary_path(dir: &Path, hash: &str) -> PathBuf {
    let name = hash.strip_prefix("sha256:").unwrap_or(hash);
    let counter = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    dir.join(format!(".{name}.{}.{}.tmp", std::process::id(), counter))
}

fn prepare_directory(dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|error| format!("create cache directory: {error}"))?;
    set_private_permissions(dir)
}

fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    set_private_file_permissions(&file)?;
    file.write_all(bytes)?;
    file.sync_all()
}

#[cfg(unix)]
fn set_private_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("set cache permissions: {error}"))
}

#[cfg(not(unix))]
fn set_private_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn set_private_file_permissions(file: &std::fs::File) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    file.set_permissions(fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn set_private_file_permissions(_file: &std::fs::File) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests;
