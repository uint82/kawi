use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use sha2::{Digest, Sha256};

use crate::history_persist::PersistedHistory;

mod format;

use format::{decode, encode_trimmed, path_bytes, MAX_FILE_BYTES, MAX_PATH_BYTES};

const TEMP_PREFIX: &str = ".undo-";

#[derive(Debug)]
pub enum UndoStoreError {
    Io(io::Error),
    Invalid(String),
    Version(u32),
    Checksum,
    TooLarge,
}

impl std::fmt::Display for UndoStoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Invalid(message) => formatter.write_str(message),
            Self::Version(version) => write!(formatter, "unsupported undo version {version}"),
            Self::Checksum => formatter.write_str("undo checksum mismatch"),
            Self::TooLarge => formatter.write_str("undo history exceeds the byte cap"),
        }
    }
}

impl std::error::Error for UndoStoreError {}

impl From<io::Error> for UndoStoreError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug, Clone)]
pub struct UndoStore {
    dir: PathBuf,
}

impl UndoStore {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn load(
        &self,
        path: &Path,
        content_hash: &[u8; 32],
    ) -> Result<Option<PersistedHistory>, UndoStoreError> {
        let canonical = canonical_path(path);
        let file = self.file_path(&canonical);
        let metadata = match fs::metadata(&file) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        if metadata.len() > (MAX_FILE_BYTES + MAX_PATH_BYTES as usize + 1024) as u64 {
            let _ = fs::remove_file(&file);
            return Err(UndoStoreError::TooLarge);
        }
        let bytes = match fs::read(&file) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        match decode(&bytes) {
            Ok(record) if record.path != path_bytes(&canonical) => {
                let _ = fs::remove_file(&file);
                Ok(None)
            }
            Ok(record) if &record.content_hash != content_hash => {
                let _ = fs::remove_file(&file);
                Ok(None)
            }
            Ok(record) => {
                touch_file(&file);
                Ok(Some(record.history))
            }
            Err(error) => {
                let _ = fs::remove_file(&file);
                Err(error)
            }
        }
    }

    pub fn save(
        &self,
        path: &Path,
        content_hash: &[u8; 32],
        history: &PersistedHistory,
    ) -> Result<(), UndoStoreError> {
        let canonical = canonical_path(path);
        self.ensure_dir()?;
        let file = self.file_path(&canonical);
        let bytes = encode_trimmed(&canonical, content_hash, history)?;
        write_private_atomic(&file, &bytes)
    }

    pub fn remove(&self, path: &Path) {
        let file = self.file_path(&canonical_path(path));
        let _ = fs::remove_file(file);
    }

    pub fn prune(&self, max_total: u64, max_age: Duration) -> Result<(), UndoStoreError> {
        let entries = match fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        let now = SystemTime::now();
        let mut files = Vec::new();
        let mut total: u64 = 0;
        for entry in entries.flatten() {
            let path = entry.path();
            if !is_undo_key(&path) {
                continue;
            }
            let metadata = match entry.metadata() {
                Ok(metadata) if metadata.is_file() => metadata,
                _ => continue,
            };
            let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let age = now.duration_since(modified).unwrap_or_default();
            total = total.saturating_add(metadata.len());
            files.push((modified, entry.path(), metadata.len(), age));
        }
        files.sort_by_key(|(modified, ..)| *modified);
        for (_, path, size, age) in files.iter().rev() {
            if *age > max_age {
                remove_and_subtract(path, size, &mut total);
            }
        }
        for (_, path, size, _) in files {
            if total <= max_total {
                break;
            }
            remove_and_subtract(&path, &size, &mut total);
        }
        Ok(())
    }

    fn ensure_dir(&self) -> Result<(), UndoStoreError> {
        fs::create_dir_all(&self.dir)?;
        set_private_dir(&self.dir)?;
        Ok(())
    }

    fn file_path(&self, canonical: &Path) -> PathBuf {
        let digest = Sha256::digest(path_bytes(canonical));
        self.dir.join(hex(&digest))
    }
}

fn write_private_atomic(path: &Path, bytes: &[u8]) -> Result<(), UndoStoreError> {
    let parent = path
        .parent()
        .ok_or_else(|| UndoStoreError::Invalid("undo file has no parent".to_string()))?;
    let temp = parent.join(format!(
        "{TEMP_PREFIX}{}-{}.tmp",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let result = write_temp(&temp, bytes)
        .and_then(|()| fs::rename(&temp, path))
        .and_then(|()| sync_directory(parent));
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result.map_err(UndoStoreError::Io)
}

fn write_temp(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    apply_private_mode(&mut options);
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

fn touch_file(path: &Path) {
    let _ = OpenOptions::new()
        .write(true)
        .open(path)
        .and_then(|file| file.set_modified(SystemTime::now()));
}

fn is_undo_key(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.len() == 64 && name.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn remove_and_subtract(path: &Path, size: &u64, total: &mut u64) {
    if fs::remove_file(path).is_ok() {
        *total = total.saturating_sub(*size);
    }
}

fn canonical_path(path: &Path) -> PathBuf {
    fs::canonicalize(path)
        .unwrap_or_else(|_| std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf()))
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(DIGITS[usize::from(byte >> 4)] as char);
        text.push(DIGITS[usize::from(byte & 0x0f)] as char);
    }
    text
}

#[cfg(unix)]
fn set_private_dir(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn set_private_dir(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn apply_private_mode(options: &mut OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;
    options.mode(0o600);
}

#[cfg(not(unix))]
fn apply_private_mode(_options: &mut OpenOptions) {}

#[cfg(test)]
mod tests;
