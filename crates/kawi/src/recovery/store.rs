use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::persist::write_private_atomically;
use crate::recovery::model::{Fingerprint, Snapshot, SnapshotHeader};
use crate::workspace;

const SNAPSHOT_EXTENSION: &str = "snap";

mod journal;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub(crate) struct WorkspaceStore {
    directory: PathBuf,
    cwd: PathBuf,
}

impl WorkspaceStore {
    pub(crate) fn create(root: &Path, cwd: &Path) -> Result<Self, String> {
        let cwd = fs::canonicalize(cwd).map_err(|error| format!("recovery: {error}"))?;
        ensure_private_directory(root)?;
        let directory = root.join(workspace::key(&cwd));
        ensure_private_directory(&directory)?;
        Ok(Self { directory, cwd })
    }

    pub(crate) fn directory(&self) -> &Path {
        &self.directory
    }

    pub(crate) fn cwd(&self) -> &Path {
        &self.cwd
    }

    pub(crate) fn save(&self, mut snapshot: Snapshot) -> Result<(), String> {
        preserve_fingerprint(
            &self.snapshot_path(&snapshot.header.buffer_id),
            &mut snapshot,
        );
        let bytes = snapshot.to_bytes()?;
        let path = self.snapshot_path(&snapshot.header.buffer_id);
        write_private_atomically(&path, &bytes).map_err(|error| format!("recovery: {error}"))
    }

    pub(crate) fn remove(&self, buffer_id: &str) -> Result<(), String> {
        let path = self.snapshot_path(buffer_id);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("recovery: {error}")),
        }
    }

    #[cfg(test)]
    pub(crate) fn clear(&self) -> Result<(), String> {
        for path in self.snapshot_paths()? {
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(format!("recovery: {error}")),
            }
        }
        Ok(())
    }

    pub(crate) fn load_all(&self) -> (Vec<Snapshot>, Vec<String>) {
        let paths = match self.snapshot_paths() {
            Ok(paths) => paths,
            Err(error) => return (Vec::new(), vec![error]),
        };
        let mut snapshots = Vec::new();
        let mut warnings = Vec::new();
        for path in paths {
            match read_snapshot(&path) {
                Ok(Some(snapshot)) => snapshots.push(snapshot),
                Ok(None) => {}
                Err(error) => warnings.push(error),
            }
        }
        (snapshots, warnings)
    }

    fn snapshot_paths(&self) -> Result<Vec<PathBuf>, String> {
        let entries = match fs::read_dir(&self.directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(format!("recovery: {error}")),
        };
        let mut paths = Vec::new();
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => return Err(format!("recovery: {error}")),
            };
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) == Some(SNAPSHOT_EXTENSION) {
                paths.push(path);
            }
        }
        paths.sort();
        Ok(paths)
    }

    fn snapshot_path(&self, buffer_id: &str) -> PathBuf {
        self.directory.join(format!(
            "{}.snap",
            workspace::digest_hex(buffer_id.as_bytes())
        ))
    }
}

fn preserve_fingerprint(path: &Path, snapshot: &mut Snapshot) {
    if snapshot.header.fingerprint.is_some() {
        return;
    }
    if let Ok(Some(existing)) = read_header(path) {
        if existing.path == snapshot.header.path {
            snapshot.header.fingerprint = existing.fingerprint;
            return;
        }
    }
    snapshot.header.fingerprint = snapshot
        .header
        .path
        .as_deref()
        .and_then(|path| Fingerprint::capture(Path::new(path)));
}

fn read_header(path: &Path) -> Result<Option<SnapshotHeader>, String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("recovery: {error}")),
    };
    let mut line = Vec::new();
    BufReader::new(file)
        .read_until(b'\n', &mut line)
        .map_err(|error| format!("recovery: {error}"))?;
    if line.last() == Some(&b'\n') {
        line.pop();
    }
    serde_json::from_slice(&line)
        .map(Some)
        .map_err(|error| format!("recovery: snapshot header: {error}"))
}

fn read_snapshot(path: &Path) -> Result<Option<Snapshot>, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "recovery: {}: {error}",
                path.file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("snapshot")
            ));
        }
    };
    Snapshot::from_bytes(&bytes).map(Some).map_err(|error| {
        format!(
            "recovery: {}: {error}",
            path.file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("snapshot")
        )
    })
}

fn ensure_private_directory(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|error| format!("recovery: {error}"))?;
    set_private_permissions(path)
}

#[cfg(unix)]
fn set_private_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .with_context(|| format!("recovery: {}", path.display()))
        .map_err(|error| error.to_string())
}

#[cfg(not(unix))]
fn set_private_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}
