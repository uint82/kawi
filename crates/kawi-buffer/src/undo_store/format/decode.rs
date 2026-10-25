use sha2::{Digest, Sha256};

use crate::history_persist::{HistoryDelta, HistoryEntry, PersistedHistory};
use crate::undo_store::UndoStoreError;

use super::{Record, MAGIC, MAX_ENTRIES, MAX_FILE_BYTES, MAX_PATH_BYTES, VERSION};

pub(crate) fn decode(bytes: &[u8]) -> Result<Record, UndoStoreError> {
    let mut reader = Reader::new(bytes);
    if reader.take(MAGIC.len())? != MAGIC {
        return Err(UndoStoreError::Invalid("invalid undo magic".to_string()));
    }
    let version = reader.u32()?;
    if version != 1 && version != VERSION {
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
    let history = match version {
        1 => decode_payload_v1(payload)?,
        _ => decode_payload_v2(payload)?,
    };
    Ok(Record {
        content_hash,
        path,
        history,
    })
}

fn decode_payload_v1(bytes: &[u8]) -> Result<PersistedHistory, UndoStoreError> {
    let mut reader = Reader::new(bytes);
    let current_id = reader.u64()?;
    let count = entry_count(&mut reader)?;
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
            timestamp: 0,
            save_marker: None,
            last_visited_child: None,
        });
    }
    finish_payload(&reader)?;
    let next_id = entries
        .iter()
        .map(|entry| entry.id)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    for index in 0..entries.len() {
        let child = entries[index].id;
        if let Some(parent) = entries[index].parent {
            if let Some(entry) = entries.iter_mut().find(|entry| entry.id == parent) {
                entry.last_visited_child = Some(child);
            }
        }
    }
    Ok(PersistedHistory {
        current_id,
        clean_id: Some(current_id),
        next_id,
        next_write_marker: 1,
        entries,
    })
}

fn decode_payload_v2(bytes: &[u8]) -> Result<PersistedHistory, UndoStoreError> {
    let mut reader = Reader::new(bytes);
    let current_id = reader.u64()?;
    let clean_id = read_optional(&mut reader)?;
    let next_id = reader.u64()?;
    let next_write_marker = reader.u64()?;
    let count = entry_count(&mut reader)?;
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
        let timestamp = reader.u64()?;
        let save_marker = read_optional(&mut reader)?;
        let last_visited_child = read_optional(&mut reader)?;
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
            timestamp,
            save_marker,
            last_visited_child,
        });
    }
    finish_payload(&reader)?;
    Ok(PersistedHistory {
        current_id,
        clean_id,
        next_id,
        next_write_marker,
        entries,
    })
}

fn entry_count(reader: &mut Reader<'_>) -> Result<usize, UndoStoreError> {
    let count = reader.u32()? as usize;
    if count == 0 || count > MAX_ENTRIES {
        return Err(UndoStoreError::Invalid(
            "invalid undo entry count".to_string(),
        ));
    }
    Ok(count)
}

fn finish_payload(reader: &Reader<'_>) -> Result<(), UndoStoreError> {
    if !reader.is_empty() {
        return Err(UndoStoreError::Invalid("trailing undo payload".to_string()));
    }
    Ok(())
}

fn read_optional(reader: &mut Reader<'_>) -> Result<Option<u64>, UndoStoreError> {
    let present = reader.u8()?;
    let value = reader.u64()?;
    match present {
        0 => Ok(None),
        1 => Ok(Some(value)),
        _ => Err(UndoStoreError::Invalid(
            "invalid undo optional value".to_string(),
        )),
    }
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

    fn u8(&mut self) -> Result<u8, UndoStoreError> {
        Ok(self.take(1)?[0])
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
