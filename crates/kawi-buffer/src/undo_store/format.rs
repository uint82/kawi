use std::path::Path;

use sha2::{Digest, Sha256};

use crate::history_persist::{HistoryDelta, HistoryEntry, PersistedHistory};

use super::UndoStoreError;

pub(super) const MAX_PATH_BYTES: u64 = 64 * 1024;
pub(super) const MAX_ENTRIES: usize = 401;
pub(super) const MAX_FILE_BYTES: usize = 2 * 1024 * 1024;

const MAGIC: &[u8; 8] = b"KAWIUNDO";
const VERSION: u32 = 1;

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
    loop {
        let bytes = encode(path, content_hash, &history)?;
        if bytes.len() <= MAX_FILE_BYTES {
            return Ok(bytes);
        }
        if !history.trim_oldest() {
            return Err(UndoStoreError::TooLarge);
        }
    }
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
    }
    Ok(payload)
}

fn push_text(payload: &mut Vec<u8>, text: &str) -> Result<(), UndoStoreError> {
    let length = u32::try_from(text.len()).map_err(|_| UndoStoreError::TooLarge)?;
    payload.extend_from_slice(&length.to_le_bytes());
    payload.extend_from_slice(text.as_bytes());
    Ok(())
}

pub(super) fn decode(bytes: &[u8]) -> Result<Record, UndoStoreError> {
    let mut reader = Reader::new(bytes);
    if reader.take(MAGIC.len())? != MAGIC {
        return Err(UndoStoreError::Invalid("invalid undo magic".to_string()));
    }
    let version = reader.u32()?;
    if version != VERSION {
        return Err(UndoStoreError::Version(version));
    }
    let content_hash = reader.array::<32>()?;
    let path_length = reader.u32()? as usize;
    if path_length as u64 > MAX_PATH_BYTES {
        return Err(UndoStoreError::Invalid("undo path is too long".to_string()));
    }
    let path = reader.take(path_length)?.to_vec();
    let payload_length = reader.u64()?;
    let checksum = reader.array::<32>()?;
    if payload_length > MAX_FILE_BYTES as u64 {
        return Err(UndoStoreError::TooLarge);
    }
    let payload = reader.take(payload_length as usize)?;
    if Sha256::digest(payload).as_slice() != checksum {
        return Err(UndoStoreError::Checksum);
    }
    if !reader.is_empty() {
        return Err(UndoStoreError::Invalid("trailing undo bytes".to_string()));
    }
    Ok(Record {
        content_hash,
        path,
        history: decode_payload(payload)?,
    })
}

fn decode_payload(bytes: &[u8]) -> Result<PersistedHistory, UndoStoreError> {
    let mut reader = Reader::new(bytes);
    let current_id = reader.u64()?;
    let count = reader.u32()? as usize;
    if count == 0 || count > MAX_ENTRIES {
        return Err(UndoStoreError::Invalid(
            "invalid undo entry count".to_string(),
        ));
    }
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let id = reader.u64()?;
        let parent = reader.u64()?;
        let row = reader.usize()?;
        let col = reader.usize()?;
        let line_changes = reader.u64()?;
        let start = reader.usize()?;
        let removed = reader.text()?;
        let inserted = reader.text()?;
        entries.push(HistoryEntry {
            id,
            parent: (parent != u64::MAX).then_some(parent),
            row,
            col,
            line_changes,
            delta: HistoryDelta {
                start,
                removed,
                inserted,
            },
        });
    }
    if !reader.is_empty() {
        return Err(UndoStoreError::Invalid("trailing undo payload".to_string()));
    }
    Ok(PersistedHistory {
        current_id,
        entries,
    })
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], UndoStoreError> {
        let end = self
            .offset
            .checked_add(length)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| UndoStoreError::Invalid("truncated undo file".to_string()))?;
        let slice = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(slice)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], UndoStoreError> {
        self.take(N)?
            .try_into()
            .map_err(|_| UndoStoreError::Invalid("truncated undo file".to_string()))
    }

    fn u32(&mut self) -> Result<u32, UndoStoreError> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, UndoStoreError> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    fn usize(&mut self) -> Result<usize, UndoStoreError> {
        usize::try_from(self.u64()?)
            .map_err(|_| UndoStoreError::Invalid("undo integer overflow".to_string()))
    }

    fn text(&mut self) -> Result<String, UndoStoreError> {
        let length = self.u32()? as usize;
        String::from_utf8(self.take(length)?.to_vec())
            .map_err(|_| UndoStoreError::Invalid("invalid undo text".to_string()))
    }

    fn is_empty(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

#[cfg(unix)]
pub(super) fn path_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
pub(super) fn path_bytes(path: &Path) -> Vec<u8> {
    path.to_string_lossy().as_bytes().to_vec()
}
