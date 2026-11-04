use std::fs;
use std::io;
use std::path::Path;
use std::time::SystemTime;

use kawi_buffer::content_hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fingerprint {
    modified: Option<SystemTime>,
    len: u64,
    hash: u64,
}

impl Fingerprint {
    pub fn read(path: &Path) -> io::Result<Self> {
        let metadata = fs::metadata(path)?;
        let bytes = fs::read(path)?;
        Ok(Self {
            modified: metadata.modified().ok(),
            len: bytes.len() as u64,
            hash: content_hash(&bytes),
        })
    }

    pub fn matches(&self, other: &Self) -> bool {
        self.len == other.len && self.hash == other.hash
    }

    pub fn modified(&self) -> Option<SystemTime> {
        self.modified
    }
}
