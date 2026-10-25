use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use ropey::Rope;
use sha2::{Digest, Sha256};

use crate::history::History;
use crate::history_persist::PersistedHistory;

static NEXT: AtomicU64 = AtomicU64::new(1);

pub(super) fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "kawi_undo_store_{}_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
        name
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub(super) fn hash(text: &str) -> [u8; 32] {
    Sha256::digest(text.as_bytes()).into()
}

pub(super) fn history(current: &str) -> PersistedHistory {
    let mut history = History::new();
    history.checkpoint(&Rope::from_str("one\n"), 0, 0, 0);
    history.checkpoint(&Rope::from_str("two\n"), 0, 0, 1);
    history.export(&Rope::from_str(current), 0, 0, 2)
}
