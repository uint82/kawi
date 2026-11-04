use std::fs::{self, Metadata};
use std::io;
use std::path::Path;
use std::time::SystemTime;

use kawi_buffer::content_hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fingerprint {
    stamp: Stamp,
    hash: u64,
}

impl Fingerprint {
    pub fn read(path: &Path) -> io::Result<Self> {
        let metadata = fs::metadata(path)?;
        Self::read_content(path, Stamp::from_metadata(&metadata))
    }

    pub fn refresh(path: &Path, previous: Option<&Self>) -> io::Result<Self> {
        let metadata = fs::metadata(path)?;
        let stamp = Stamp::from_metadata(&metadata);
        if let Some(previous) = previous.filter(|previous| previous.stamp == stamp) {
            return Ok(*previous);
        }
        Self::read_content(path, stamp)
    }

    pub fn matches(&self, other: &Self) -> bool {
        self.stamp.len == other.stamp.len && self.hash == other.hash
    }

    pub fn modified(&self) -> Option<SystemTime> {
        self.stamp.modified
    }

    fn read_content(path: &Path, stamp: Stamp) -> io::Result<Self> {
        let bytes = fs::read(path)?;
        Ok(Self {
            stamp,
            hash: content_hash(&bytes),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Stamp {
    len: u64,
    modified: Option<SystemTime>,
    ctime: Option<(i64, i64)>,
}

impl Stamp {
    fn from_metadata(metadata: &Metadata) -> Self {
        Self {
            len: metadata.len(),
            modified: metadata.modified().ok(),
            ctime: ctime(metadata),
        }
    }
}

#[cfg(unix)]
fn ctime(metadata: &Metadata) -> Option<(i64, i64)> {
    use std::os::unix::fs::MetadataExt;

    Some((metadata.ctime(), metadata.ctime_nsec()))
}

#[cfg(not(unix))]
fn ctime(_metadata: &Metadata) -> Option<(i64, i64)> {
    None
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    fn temp_file(name: &str) -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "kawi_watch_fingerprint_{}_{}",
            std::process::id(),
            name
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("directory");
        let path = dir.join("file.txt");
        fs::write(&path, "alpha\n").expect("write");
        (dir, path)
    }

    #[test]
    fn refresh_reuses_matching_metadata() {
        let (dir, path) = temp_file("reuse");
        let metadata = fs::metadata(&path).expect("metadata");
        let previous = Fingerprint {
            stamp: Stamp::from_metadata(&metadata),
            hash: u64::MAX,
        };
        let refreshed = Fingerprint::refresh(&path, Some(&previous)).expect("refresh");
        assert_eq!(refreshed, previous);
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn refresh_hashes_changed_metadata() {
        let (dir, path) = temp_file("changed");
        let metadata = fs::metadata(&path).expect("metadata");
        let previous = Fingerprint {
            stamp: Stamp::from_metadata(&metadata),
            hash: u64::MAX,
        };
        fs::write(&path, "beta and a longer body\n").expect("change");
        let refreshed = Fingerprint::refresh(&path, Some(&previous)).expect("refresh");
        assert_ne!(refreshed, previous);
        assert!(refreshed.matches(&Fingerprint::read(&path).expect("read")));
        fs::remove_dir_all(dir).expect("cleanup");
    }
}
