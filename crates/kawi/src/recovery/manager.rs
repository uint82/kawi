use std::collections::BTreeMap;
use std::path::Path;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use crate::recovery::lock::{self, Lock, LockState};
use crate::recovery::model::Snapshot;
use crate::recovery::store::WorkspaceStore;
use crate::recovery::writer::SnapshotWriter;

mod directory;

const IDLE_DELAY: Duration = Duration::from_millis(1_500);
const MAX_DELAY: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub(crate) struct BufferObservation {
    pub number: usize,
    pub id: String,
    pub name: String,
    pub path: Option<String>,
    pub revision: u64,
    pub dirty: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingBuffer {
    pub number: usize,
    pub id: String,
    pub name: String,
    pub path: Option<String>,
    pub revision: u64,
}

pub(crate) enum RecoveryStart {
    Disabled(String),
    Active {
        manager: Box<RecoveryManager>,
        snapshots: Vec<Snapshot>,
        warning: Option<String>,
    },
}

struct TrackedBuffer {
    number: usize,
    name: String,
    path: Option<String>,
    revision: u64,
    snapshot_revision: Option<u64>,
}

pub(crate) struct RecoveryManager {
    store: WorkspaceStore,
    lock: Option<Lock>,
    writer: SnapshotWriter,
    tracked: BTreeMap<String, TrackedBuffer>,
    last_change: Option<Instant>,
    pending_since: Option<Instant>,
    active: bool,
}

impl RecoveryManager {
    pub(crate) fn start(root: &Path, cwd: &Path, wake: Sender<()>) -> RecoveryStart {
        let store = match WorkspaceStore::create(root, cwd) {
            Ok(store) => store,
            Err(warning) => return RecoveryStart::Disabled(warning),
        };
        let lock = match lock::acquire(store.directory()) {
            Ok(LockState::Acquired(lock)) => lock,
            Ok(LockState::Busy(pid)) => {
                return RecoveryStart::Disabled(format!(
                    "recovery disabled: workspace is in use by PID {pid}"
                ));
            }
            Err(error) => {
                return RecoveryStart::Disabled(format!("recovery disabled: {error}"));
            }
        };
        let (snapshots, warnings) = store.load_all();
        let writer = SnapshotWriter::new(store.clone(), wake);
        RecoveryStart::Active {
            manager: Box::new(Self {
                store,
                lock: Some(lock),
                writer,
                tracked: BTreeMap::new(),
                last_change: None,
                pending_since: None,
                active: true,
            }),
            snapshots,
            warning: (!warnings.is_empty()).then(|| warnings.join("\n")),
        }
    }

    pub(crate) fn observe(&mut self, buffers: &[BufferObservation], now: Instant) -> bool {
        if !self.active {
            return false;
        }
        let was_pending = self.has_pending();
        let mut changed = false;
        let mut removed = Vec::new();
        let observed: BTreeMap<&str, &BufferObservation> = buffers
            .iter()
            .map(|buffer| (buffer.id.as_str(), buffer))
            .collect();
        for (id, tracked) in &mut self.tracked {
            let Some(buffer) = observed.get(id.as_str()) else {
                removed.push(id.clone());
                continue;
            };
            if !buffer.dirty {
                removed.push(id.clone());
                continue;
            }
            tracked.number = buffer.number;
            tracked.name.clone_from(&buffer.name);
            tracked.path.clone_from(&buffer.path);
            if tracked.revision != buffer.revision {
                tracked.revision = buffer.revision;
                changed = true;
            }
        }
        for id in removed {
            self.tracked.remove(&id);
            self.writer.remove(id);
        }
        for buffer in buffers.iter().filter(|buffer| buffer.dirty) {
            if self.tracked.contains_key(&buffer.id) {
                continue;
            }
            self.tracked.insert(
                buffer.id.clone(),
                TrackedBuffer {
                    number: buffer.number,
                    name: buffer.name.clone(),
                    path: buffer.path.clone(),
                    revision: buffer.revision,
                    snapshot_revision: None,
                },
            );
            changed = true;
        }
        let pending = self.has_pending();
        if pending {
            if changed || self.last_change.is_none() {
                self.last_change = Some(now);
            }
            if !was_pending || self.pending_since.is_none() {
                self.pending_since = Some(now);
            }
        } else {
            self.last_change = None;
            self.pending_since = None;
        }
        changed
    }

    pub(crate) fn due(&self, now: Instant) -> bool {
        self.deadline(now) == Some(Duration::ZERO)
    }

    pub(crate) fn deadline(&self, now: Instant) -> Option<Duration> {
        if !self.has_pending() {
            return None;
        }
        let idle = self.last_change.unwrap_or(now) + IDLE_DELAY;
        let limit = self.pending_since.unwrap_or(now) + MAX_DELAY;
        Some(idle.min(limit).saturating_duration_since(now))
    }

    pub(crate) fn pending_buffers(&self) -> Vec<PendingBuffer> {
        self.tracked
            .iter()
            .filter_map(|(id, tracked)| {
                if tracked.snapshot_revision == Some(tracked.revision) {
                    return None;
                }
                Some(PendingBuffer {
                    number: tracked.number,
                    id: id.clone(),
                    name: tracked.name.clone(),
                    path: tracked.path.clone(),
                    revision: tracked.revision,
                })
            })
            .collect()
    }

    pub(crate) fn queue(&mut self, snapshots: Vec<Snapshot>) -> bool {
        if snapshots.is_empty() {
            return true;
        }
        let queued: Vec<(String, u64)> = snapshots
            .iter()
            .map(|snapshot| (snapshot.header.buffer_id.clone(), snapshot.header.revision))
            .collect();
        if !self.writer.submit(snapshots) {
            return false;
        }
        for (buffer_id, revision) in queued {
            if let Some(tracked) = self.tracked.get_mut(&buffer_id) {
                tracked.snapshot_revision = Some(revision);
            }
        }
        if !self.has_pending() {
            self.last_change = None;
            self.pending_since = None;
        }
        true
    }

    pub(crate) fn drain_errors(&self) -> Vec<String> {
        self.writer
            .drain()
            .into_iter()
            .filter_map(|result| {
                result
                    .error
                    .map(|error| format!("{error} (buffer {})", result.buffer_id))
            })
            .collect()
    }

    pub(crate) fn track_recovered(
        &mut self,
        buffer_id: String,
        number: usize,
        name: String,
        path: Option<String>,
        revision: u64,
    ) {
        self.tracked.insert(
            buffer_id,
            TrackedBuffer {
                number,
                name,
                path,
                revision,
                snapshot_revision: Some(revision),
            },
        );
    }

    pub(crate) fn remove_snapshot(&mut self, buffer_id: &str) -> Option<String> {
        self.tracked.remove(buffer_id);
        self.writer.remove(buffer_id.to_string());
        self.store.remove(buffer_id).err()
    }

    pub(crate) fn finish_clean(&mut self, buffers: &[BufferObservation]) {
        if !self.active {
            return;
        }
        self.writer.stop();
        self.remove_clean_snapshots(buffers);
        self.release_lock();
        self.active = false;
    }

    pub(crate) fn finish_abnormal(
        &mut self,
        buffers: &[BufferObservation],
        snapshots: Vec<Snapshot>,
    ) -> Vec<String> {
        if !self.active {
            return Vec::new();
        }
        self.writer.stop();
        self.remove_clean_snapshots(buffers);
        let errors = self.save_sync(snapshots);
        self.keep_lock();
        self.active = false;
        errors
    }

    pub(crate) fn discard_buffers(&mut self, buffer_ids: &[String]) -> Option<String> {
        if !self.active {
            return None;
        }
        self.writer.stop();
        let mut errors = Vec::new();
        for buffer_id in buffer_ids {
            self.tracked.remove(buffer_id);
            if let Some(error) = self.store.remove(buffer_id).err() {
                errors.push(error);
            }
        }
        self.release_lock();
        self.active = false;
        (!errors.is_empty()).then(|| errors.join("\n"))
    }

    pub(crate) fn cwd(&self) -> &Path {
        self.store.cwd()
    }

    pub(crate) fn store_clone(&self) -> WorkspaceStore {
        self.store.clone()
    }

    pub(crate) fn tracked_ids(&self) -> Vec<String> {
        self.tracked.keys().cloned().collect()
    }

    fn save_sync(&self, snapshots: Vec<Snapshot>) -> Vec<String> {
        snapshots
            .into_iter()
            .filter_map(|snapshot| self.store.save(snapshot).err())
            .collect()
    }

    fn remove_clean_snapshots(&mut self, buffers: &[BufferObservation]) {
        let dirty: BTreeMap<&str, bool> = buffers
            .iter()
            .map(|buffer| (buffer.id.as_str(), buffer.dirty))
            .collect();
        let clean: Vec<String> = self
            .tracked
            .iter()
            .filter(|(id, _)| dirty.get(id.as_str()) != Some(&true))
            .map(|(id, _)| id.clone())
            .collect();
        for id in clean {
            self.tracked.remove(&id);
            let _ = self.store.remove(&id);
        }
    }

    fn has_pending(&self) -> bool {
        self.tracked
            .values()
            .any(|tracked| tracked.snapshot_revision != Some(tracked.revision))
    }

    fn release_lock(&mut self) {
        if let Some(mut lock) = self.lock.take() {
            lock.release();
        }
    }

    fn keep_lock(&mut self) {
        if let Some(mut lock) = self.lock.take() {
            lock.keep();
        }
    }
}

impl Drop for RecoveryManager {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        self.writer.stop();
        self.keep_lock();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recovery::model::{SnapshotHeader, SNAPSHOT_VERSION};
    use crate::session::SessionView;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::mpsc;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kawi_recovery_manager_{}_{}",
            std::process::id(),
            name
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn observation(id: &str, revision: u64, dirty: bool) -> BufferObservation {
        BufferObservation {
            number: 1,
            id: id.to_string(),
            name: "[No Name]".to_string(),
            path: None,
            revision,
            dirty,
        }
    }

    fn start(root: &Path) -> RecoveryManager {
        let (wake, _wake) = mpsc::channel();
        match RecoveryManager::start(root, root, wake) {
            RecoveryStart::Active { manager, .. } => *manager,
            RecoveryStart::Disabled(warning) => panic!("{warning}"),
        }
    }

    fn snapshot(id: &str, revision: u64) -> Snapshot {
        Snapshot {
            header: SnapshotHeader {
                version: SNAPSHOT_VERSION,
                buffer_id: id.to_string(),
                name: "[No Name]".to_string(),
                path: None,
                revision,
                fingerprint: None,
                view: SessionView {
                    row: 0,
                    col: 0,
                    desired: 0,
                    top: 0,
                    leftcol: 0,
                },
            },
            text: "dirty".to_string(),
        }
    }

    #[test]
    fn live_workspaces_disable_recovery() {
        let root = temp_dir("busy");
        let _manager = start(&root);
        let (wake, _wake) = mpsc::channel();
        match RecoveryManager::start(&root, &root, wake) {
            RecoveryStart::Disabled(warning) => {
                assert!(warning.contains("in use by PID"));
            }
            RecoveryStart::Active { .. } => panic!("expected a disabled manager"),
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn idle_debounce_and_max_wait_are_enforced() {
        let root = temp_dir("timing");
        let mut manager = start(&root);
        let start = Instant::now();
        manager.observe(&[observation("buffer", 1, true)], start);
        assert!(!manager.due(start + IDLE_DELAY - Duration::from_millis(1)));
        assert!(manager.due(start + IDLE_DELAY));
        manager.observe(
            &[observation("buffer", 2, true)],
            start + Duration::from_secs(1),
        );
        manager.observe(
            &[observation("buffer", 3, true)],
            start + Duration::from_secs(5),
        );
        assert!(manager.due(start + MAX_DELAY));
        manager.finish_clean(&[observation("buffer", 3, true)]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unchanged_revisions_are_not_queued() {
        let root = temp_dir("unchanged");
        let mut manager = start(&root);
        let now = Instant::now();
        manager.observe(&[observation("buffer", 1, true)], now);
        assert!(manager.queue(vec![snapshot("buffer", 1)]));
        manager.observe(
            &[observation("buffer", 1, true)],
            now + Duration::from_secs(2),
        );
        assert!(manager.pending_buffers().is_empty());
        manager.observe(
            &[observation("buffer", 2, true)],
            now + Duration::from_millis(2),
        );
        assert_eq!(manager.pending_buffers().len(), 1);
        manager.finish_clean(&[observation("buffer", 2, true)]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn clean_buffers_lose_their_snapshots() {
        let root = temp_dir("cleanup");
        let mut manager = start(&root);
        let now = Instant::now();
        manager.observe(&[observation("buffer", 1, true)], now);
        assert!(manager.queue(vec![snapshot("buffer", 1)]));
        manager.finish_clean(&[observation("buffer", 1, false)]);
        let store = WorkspaceStore::create(&root, &root).unwrap();
        assert!(store.load_all().0.is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
