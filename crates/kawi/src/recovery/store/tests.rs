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
