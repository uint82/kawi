use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{
    canonical_path, decode, encode_trimmed, is_undo_key, path_bytes, write_private_atomic,
};
use super::{UndoStore, UndoStoreError};

impl UndoStore {
    pub fn move_path(&self, old: &Path, new: &Path) -> Result<(), UndoStoreError> {
        self.move_prefix(old, new)
    }

    pub fn move_prefix(&self, old: &Path, new: &Path) -> Result<(), UndoStoreError> {
        self.rewrite_prefix(old, new, false)
    }

    pub fn remove_prefix(&self, old: &Path) -> Result<(), UndoStoreError> {
        self.rewrite_prefix(old, old, true)
    }

    fn rewrite_prefix(&self, old: &Path, new: &Path, remove: bool) -> Result<(), UndoStoreError> {
        let entries = match fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        let old_path = canonical_path(old);
        let new_path = canonical_path(new);
        let old_bytes = path_bytes(&old_path);
        let new_bytes = path_bytes(&new_path);
        for entry in entries.flatten() {
            let file = entry.path();
            if !is_undo_key(&file) {
                continue;
            }
            let bytes = match fs::read(&file) {
                Ok(bytes) => bytes,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            };
            let record = match decode(&bytes) {
                Ok(record) => record,
                Err(_) => continue,
            };
            let Some(rewritten) = rewrite_record_path(&record.path, &old_bytes, &new_bytes) else {
                continue;
            };
            if remove {
                let _ = fs::remove_file(&file);
                continue;
            }
            let path = bytes_to_path(&rewritten);
            let target = self.file_path(&path);
            let encoded = encode_trimmed(&path, &record.content_hash, &record.history)?;
            write_private_atomic(&target, &encoded)?;
            if target != file {
                let _ = fs::remove_file(&file);
            }
        }
        Ok(())
    }
}

fn rewrite_record_path(path: &[u8], old: &[u8], new: &[u8]) -> Option<Vec<u8>> {
    if path == old {
        return Some(new.to_vec());
    }
    let rest = path.strip_prefix(old)?;
    let separator = rest.first()?;
    if *separator != b'/' {
        return None;
    }
    let mut rewritten = Vec::with_capacity(new.len().saturating_add(rest.len()));
    rewritten.extend_from_slice(new);
    rewritten.extend_from_slice(rest);
    Some(rewritten)
}

#[cfg(unix)]
fn bytes_to_path(bytes: &[u8]) -> PathBuf {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(OsStr::from_bytes(bytes))
}

#[cfg(not(unix))]
fn bytes_to_path(bytes: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
}
