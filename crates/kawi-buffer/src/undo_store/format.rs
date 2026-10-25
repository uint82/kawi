use std::path::Path;

use sha2::{Digest, Sha256};

use crate::history_persist::PersistedHistory;

use super::UndoStoreError;

mod decode;

pub(super) use decode::decode;

pub(super) const MAX_PATH_BYTES: u64 = 64 * 1024;
pub(super) const MAX_ENTRIES: usize = 401;
pub(super) const MAX_FILE_BYTES: usize = 2 * 1024 * 1024;

pub(super) const MAGIC: &[u8; 8] = b"KAWIUNDO";
pub(super) const VERSION: u32 = 2;

pub(super) struct Record {
    pub(super) content_hash: [u8; 32],
    pub(super) path: Vec<u8>,
    pub(super) history: PersistedHistory,
}

pub(super) fn encode_trimmed(
    path: &Path,
    content_hash: &[u8; 32],
    history: &PersistedHistory,
) -> Result<Vec<u8>, UndoStoreError> {
    let mut history = history.clone();
    while encoded_len(path, &history)? > MAX_FILE_BYTES {
        if !history.trim_oldest() {
            return Err(UndoStoreError::TooLarge);
        }
    }
    encode(path, content_hash, &history)
}

fn encoded_len(path: &Path, history: &PersistedHistory) -> Result<usize, UndoStoreError> {
    let path_length = path_bytes(path).len();
    if path_length as u64 > MAX_PATH_BYTES {
        return Err(UndoStoreError::TooLarge);
    }
    let payload = payload_len(history)?;
    let total = 8u64 + 4 + 32 + 4 + path_length as u64 + 8 + 32 + payload;
    usize::try_from(total).map_err(|_| UndoStoreError::TooLarge)
}

fn payload_len(history: &PersistedHistory) -> Result<u64, UndoStoreError> {
    if history.entries.len() > MAX_ENTRIES {
        return Err(UndoStoreError::TooLarge);
    }
    let mut length = 8u64 + 1 + 8 + 8 + 8 + 4;
    for entry in &history.entries {
        length = length
            .checked_add(82)
            .and_then(|value| value.checked_add(entry.delta.removed.len() as u64))
            .and_then(|value| value.checked_add(entry.delta.inserted.len() as u64))
            .ok_or(UndoStoreError::TooLarge)?;
    }
    Ok(length)
}

fn encode(
    path: &Path,
    content_hash: &[u8; 32],
    history: &PersistedHistory,
) -> Result<Vec<u8>, UndoStoreError> {
    let path = path_bytes(path);
    if path.len() as u64 > MAX_PATH_BYTES {
        return Err(UndoStoreError::TooLarge);
    }
    let payload = encode_payload(history)?;
    let mut bytes = Vec::with_capacity(8 + 4 + 32 + 4 + path.len() + 8 + 32 + payload.len());
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&VERSION.to_le_bytes());
    bytes.extend_from_slice(content_hash);
    bytes.extend_from_slice(&(path.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&path);
    bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&Sha256::digest(&payload));
    bytes.extend_from_slice(&payload);
    Ok(bytes)
}

fn encode_payload(history: &PersistedHistory) -> Result<Vec<u8>, UndoStoreError> {
    if history.entries.len() > MAX_ENTRIES {
        return Err(UndoStoreError::TooLarge);
    }
    let mut payload = Vec::new();
    payload.extend_from_slice(&history.current_id.to_le_bytes());
    push_optional(&mut payload, history.clean_id);
    payload.extend_from_slice(&history.next_id.to_le_bytes());
    payload.extend_from_slice(&history.next_write_marker.to_le_bytes());
    payload.extend_from_slice(&(history.entries.len() as u32).to_le_bytes());
    for entry in &history.entries {
        payload.extend_from_slice(&entry.id.to_le_bytes());
        payload.extend_from_slice(&entry.parent.unwrap_or(u64::MAX).to_le_bytes());
        payload.extend_from_slice(&(entry.row as u64).to_le_bytes());
        payload.extend_from_slice(&(entry.col as u64).to_le_bytes());
        payload.extend_from_slice(&entry.line_changes.to_le_bytes());
        payload.extend_from_slice(&(entry.delta.start as u64).to_le_bytes());
        push_text(&mut payload, &entry.delta.removed)?;
        push_text(&mut payload, &entry.delta.inserted)?;
        payload.extend_from_slice(&entry.timestamp.to_le_bytes());
        push_optional(&mut payload, entry.save_marker);
        push_optional(&mut payload, entry.last_visited_child);
    }
    Ok(payload)
}

fn push_optional(payload: &mut Vec<u8>, value: Option<u64>) {
    payload.push(u8::from(value.is_some()));
    payload.extend_from_slice(&value.unwrap_or_default().to_le_bytes());
}

fn push_text(payload: &mut Vec<u8>, text: &str) -> Result<(), UndoStoreError> {
    let length = u32::try_from(text.len()).map_err(|_| UndoStoreError::TooLarge)?;
    payload.extend_from_slice(&length.to_le_bytes());
    payload.extend_from_slice(text.as_bytes());
    Ok(())
}

pub(super) fn path_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
pub(super) fn path_bytes(path: &Path) -> Vec<u8> {
    path.to_string_lossy().as_bytes().to_vec()
}
