use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Mutex, Once, OnceLock};

use ropey::Rope;

use crate::recovery::model::{Snapshot, SnapshotHeader};
use crate::recovery::store::WorkspaceStore;

pub(crate) struct PanicSnapshot {
    pub header: SnapshotHeader,
    pub text: Rope,
}

struct PanicState {
    store: WorkspaceStore,
    snapshots: Vec<PanicSnapshot>,
}

static STATES: OnceLock<Mutex<BTreeMap<PathBuf, PanicState>>> = OnceLock::new();
static INSTALL: Once = Once::new();

pub(crate) fn install() {
    INSTALL.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            flush();
            previous(info);
        }));
    });
}

pub(crate) fn register(store: WorkspaceStore) {
    let key = store.directory().to_path_buf();
    states()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .insert(
            key,
            PanicState {
                store,
                snapshots: Vec::new(),
            },
        );
}

pub(crate) fn update(store: &WorkspaceStore, snapshots: Vec<PanicSnapshot>) {
    let key = store.directory().to_path_buf();
    if let Some(state) = states()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .get_mut(&key)
    {
        state.snapshots = snapshots;
    }
}

pub(crate) fn clear(store: &WorkspaceStore) {
    let key = store.directory().to_path_buf();
    states()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .remove(&key);
}

pub(crate) fn flush() {
    let Ok(states) = states().try_lock() else {
        return;
    };
    for state in states.values() {
        for snapshot in &state.snapshots {
            let snapshot = Snapshot {
                header: snapshot.header.clone(),
                text: snapshot.text.to_string(),
            };
            let _ = state.store.save(snapshot);
        }
    }
}

fn states() -> &'static Mutex<BTreeMap<PathBuf, PanicState>> {
    STATES.get_or_init(|| Mutex::new(BTreeMap::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recovery::SNAPSHOT_VERSION;
    use crate::session::SessionView;
    use std::fs;
    use std::path::Path;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kawi_recovery_panic_{}_{}",
            std::process::id(),
            name
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn panic_flush_writes_cached_dirty_buffers() {
        let root = temp_dir("flush");
        let store = WorkspaceStore::create(&root, &root).unwrap();
        register(store.clone());
        update(
            &store,
            vec![PanicSnapshot {
                header: SnapshotHeader {
                    version: SNAPSHOT_VERSION,
                    buffer_id: "buffer".to_string(),
                    name: "[No Name]".to_string(),
                    path: None,
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
                text: Rope::from_str("dirty"),
            }],
        );
        flush();
        let (snapshots, warnings) = store.load_all();
        assert!(warnings.is_empty());
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].text, "dirty");
        clear(&store);
        assert!(!Path::new(&root).join("missing").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
