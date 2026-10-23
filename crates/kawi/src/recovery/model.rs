use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::session::SessionView;
use crate::workspace;

pub(crate) const SNAPSHOT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Fingerprint {
    pub modified_ns: u64,
    pub size: u64,
    pub hash: String,
}

impl Fingerprint {
    pub(crate) fn capture(path: &Path) -> Option<Self> {
        let bytes = fs::read(path).ok()?;
        let metadata = fs::metadata(path).ok()?;
        let modified_ns = modified_ns(&metadata)?;
        Some(Self {
            modified_ns,
            size: metadata.len(),
            hash: workspace::digest_hex(&bytes),
        })
    }

    pub(crate) fn matches(&self, other: &Self) -> bool {
        self.modified_ns == other.modified_ns && self.size == other.size && self.hash == other.hash
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SnapshotHeader {
    pub version: u32,
    pub buffer_id: String,
    pub name: String,
    pub path: Option<String>,
    pub revision: u64,
    pub fingerprint: Option<Fingerprint>,
    pub view: SessionView,
}

#[derive(Debug, Clone)]
pub(crate) struct Snapshot {
    pub header: SnapshotHeader,
    pub text: String,
}

impl Snapshot {
    pub(crate) fn to_bytes(&self) -> Result<Vec<u8>, String> {
        let header = serde_json::to_vec(&self.header)
            .map_err(|error| format!("snapshot header: {error}"))?;
        let mut bytes = Vec::with_capacity(header.len().saturating_add(self.text.len() + 1));
        bytes.extend_from_slice(&header);
        bytes.push(b'\n');
        bytes.extend_from_slice(self.text.as_bytes());
        Ok(bytes)
    }

    pub(crate) fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let newline = bytes
            .iter()
            .position(|byte| *byte == b'\n')
            .ok_or_else(|| "snapshot header is missing".to_string())?;
        let header: SnapshotHeader = serde_json::from_slice(&bytes[..newline])
            .map_err(|error| format!("snapshot header: {error}"))?;
        validate_header(&header)?;
        let text = std::str::from_utf8(&bytes[newline + 1..])
            .map_err(|error| format!("snapshot text: {error}"))?
            .to_string();
        Ok(Self { header, text })
    }
}

fn validate_header(header: &SnapshotHeader) -> Result<(), String> {
    if header.version != SNAPSHOT_VERSION {
        return Err(format!("unsupported snapshot version {}", header.version));
    }
    if header.buffer_id.is_empty() {
        return Err("snapshot has no buffer id".to_string());
    }
    if header.path.as_deref() == Some("") {
        return Err("snapshot has an empty path".to_string());
    }
    Ok(())
}

fn modified_ns(metadata: &fs::Metadata) -> Option<u64> {
    let duration = metadata.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    u64::try_from(duration.as_nanos()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kawi_recovery_model_{}_{}",
            std::process::id(),
            name
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn snapshot() -> Snapshot {
        Snapshot {
            header: SnapshotHeader {
                version: SNAPSHOT_VERSION,
                buffer_id: "buffer".to_string(),
                name: "alpha.txt".to_string(),
                path: Some("/tmp/alpha.txt".to_string()),
                revision: 4,
                fingerprint: Some(Fingerprint {
                    modified_ns: 1,
                    size: 2,
                    hash: "hash".to_string(),
                }),
                view: SessionView {
                    row: 1,
                    col: 2,
                    desired: 2,
                    top: 0,
                    leftcol: 0,
                },
            },
            text: "alpha\nbeta\n".to_string(),
        }
    }

    #[test]
    fn snapshots_round_trip_with_raw_text() {
        let snapshot = snapshot();
        assert_eq!(
            Snapshot::from_bytes(&snapshot.to_bytes().unwrap())
                .unwrap()
                .text,
            snapshot.text
        );
    }

    #[test]
    fn fingerprints_detect_content_changes() {
        let dir = temp_dir("fingerprint");
        let path = dir.join("alpha.txt");
        fs::write(&path, "alpha\n").unwrap();
        let first = Fingerprint::capture(&path).expect("fingerprint");
        let second = Fingerprint::capture(&path).expect("fingerprint");
        assert!(first.matches(&second));
        fs::write(&path, "beta\n").unwrap();
        let changed = Fingerprint::capture(&path).expect("fingerprint");
        assert!(!first.matches(&changed));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn corrupt_snapshots_are_rejected() {
        assert!(Snapshot::from_bytes(b"not-json").is_err());
        assert!(Snapshot::from_bytes(b"{}\ntext").is_err());
    }
}
