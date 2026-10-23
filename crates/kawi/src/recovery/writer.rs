use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use crate::recovery::model::Snapshot;
use crate::recovery::store::WorkspaceStore;

#[derive(Debug)]
pub(crate) struct WriteResult {
    pub buffer_id: String,
    pub error: Option<String>,
}

enum Message {
    Batch(Vec<Snapshot>),
    Remove(String),
    Stop,
}

pub(crate) struct SnapshotWriter {
    sender: Sender<Message>,
    results: Receiver<WriteResult>,
    handle: Option<JoinHandle<()>>,
}

impl SnapshotWriter {
    pub(crate) fn new(store: WorkspaceStore, wake: Sender<()>) -> Self {
        let (sender, messages) = mpsc::channel();
        let (result_sender, results) = mpsc::channel();
        let handle = thread::Builder::new()
            .name("kawi-recovery".to_string())
            .spawn(move || run_writer(store, messages, result_sender, wake))
            .ok();
        Self {
            sender,
            results,
            handle,
        }
    }

    pub(crate) fn submit(&self, snapshots: Vec<Snapshot>) -> bool {
        self.sender.send(Message::Batch(snapshots)).is_ok()
    }

    pub(crate) fn remove(&self, buffer_id: String) -> bool {
        self.sender.send(Message::Remove(buffer_id)).is_ok()
    }

    pub(crate) fn drain(&self) -> Vec<WriteResult> {
        self.results.try_iter().collect()
    }

    pub(crate) fn stop(&mut self) {
        let _ = self.sender.send(Message::Stop);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for SnapshotWriter {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run_writer(
    store: WorkspaceStore,
    messages: Receiver<Message>,
    results: Sender<WriteResult>,
    wake: Sender<()>,
) {
    while let Ok(message) = messages.recv() {
        match message {
            Message::Batch(snapshots) => {
                for snapshot in snapshots {
                    let buffer_id = snapshot.header.buffer_id.clone();
                    let error = store.save(snapshot).err();
                    if results.send(WriteResult { buffer_id, error }).is_err() {
                        return;
                    }
                    if wake.send(()).is_err() {
                        return;
                    }
                }
            }
            Message::Remove(buffer_id) => {
                let _ = store.remove(&buffer_id);
                if wake.send(()).is_err() {
                    return;
                }
            }
            Message::Stop => return,
        }
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
    use std::time::{Duration, Instant};

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kawi_recovery_writer_{}_{}",
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
                name: "[No Name]".to_string(),
                path: None,
                revision: 2,
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

    fn wait_for_result(writer: &SnapshotWriter) -> WriteResult {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if let Some(result) = writer.drain().into_iter().next() {
                return result;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("writer result timed out");
    }

    #[test]
    fn writes_snapshots_off_the_calling_thread() {
        let root = temp_dir("write");
        let store = WorkspaceStore::create(&root, &root).unwrap();
        let (wake, _wake) = mpsc::channel();
        let mut writer = SnapshotWriter::new(store.clone(), wake);
        assert!(writer.submit(vec![snapshot()]));
        let result = wait_for_result(&writer);
        assert!(result.error.is_none());
        assert_eq!(store.load_all().0.len(), 1);
        writer.stop();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn queued_removals_follow_writes() {
        let root = temp_dir("remove");
        let store = WorkspaceStore::create(&root, &root).unwrap();
        let (wake, _wake) = mpsc::channel();
        let mut writer = SnapshotWriter::new(store.clone(), wake);
        assert!(writer.submit(vec![snapshot()]));
        assert!(writer.remove("buffer".to_string()));
        writer.stop();
        assert!(store.load_all().0.is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
