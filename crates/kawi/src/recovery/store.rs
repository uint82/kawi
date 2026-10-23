use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::persist::write_private_atomically;
use crate::recovery::model::{Fingerprint, Snapshot, SnapshotHeader};
use crate::workspace;

const SNAPSHOT_EXTENSION: &str = "snap";

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recovery::model::{SnapshotHeader, SNAPSHOT_VERSION};
    use crate::session::SessionView;
    use std::fs;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kawi_recovery_store_{}_{}",
            std::process::id(),
            name
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn snapshot(buffer_id: &str, path: Option<&str>, text: &str) -> Snapshot {
        Snapshot {
            header: SnapshotHeader {
                version: SNAPSHOT_VERSION,
                buffer_id: buffer_id.to_string(),
                name: path.unwrap_or("[No Name]").to_string(),
                path: path.map(str::to_string),
                revision: 1,
                fingerprint: None,
                view: SessionView {
                    row: 0,
                    col: 0,
                    desired: 0,
                    top: 0,
                    leftcol: 0,
                },
            },
            text: text.to_string(),
        }
    }

    #[test]
    fn snapshots_are_per_buffer_and_clear_individually() {
        let root = temp_dir("individual");
        let store = WorkspaceStore::create(&root, &root).unwrap();
        store.save(snapshot("first", None, "alpha")).unwrap();
        store.save(snapshot("second", None, "beta")).unwrap();
        let (snapshots, warnings) = store.load_all();
        assert!(warnings.is_empty());
        assert_eq!(snapshots.len(), 2);
        store.remove("first").unwrap();
        let (snapshots, _) = store.load_all();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].header.buffer_id, "second");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn creating_the_store_uses_private_permissions() {
        let root = temp_dir("permissions");
        let store = WorkspaceStore::create(&root, &root).unwrap();
        store.save(snapshot("first", None, "alpha")).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let directory = fs::metadata(store.directory())
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            let file = fs::metadata(store.snapshot_path("first"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(directory, 0o700);
            assert_eq!(file, 0o600);
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn fingerprints_are_preserved_across_snapshot_updates() {
        let root = temp_dir("fingerprint");
        let path = root.join("alpha.txt");
        fs::write(&path, "alpha\n").unwrap();
        let store = WorkspaceStore::create(&root, &root).unwrap();
        let path_text = path.to_string_lossy().into_owned();
        store
            .save(snapshot("first", Some(&path_text), "dirty"))
            .unwrap();
        let (loaded, _) = store.load_all();
        let first = loaded[0].header.fingerprint.clone().unwrap();
        fs::write(&path, "changed\n").unwrap();
        store
            .save(snapshot("first", Some(&path_text), "dirty again"))
            .unwrap();
        let (loaded, _) = store.load_all();
        assert_eq!(loaded[0].header.fingerprint, Some(first));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn clear_removes_every_snapshot() {
        let root = temp_dir("clear");
        let store = WorkspaceStore::create(&root, &root).unwrap();
        store.save(snapshot("first", None, "alpha")).unwrap();
        store.save(snapshot("second", None, "beta")).unwrap();
        store.clear().unwrap();
        assert!(store.load_all().0.is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
