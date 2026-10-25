use super::*;
use crate::history::History;
use ropey::Rope;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(1);

fn temp_dir(name: &str) -> PathBuf {
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

fn history(current: &str) -> PersistedHistory {
    let mut history = History::new();
    history.checkpoint(&Rope::from_str("one\n"), 0, 0, 0);
    history.checkpoint(&Rope::from_str("two\n"), 0, 0, 1);
    history.export(&Rope::from_str(current), 0, 0, 2)
}

fn hash(text: &str) -> [u8; 32] {
    Sha256::digest(text.as_bytes()).into()
}

#[test]
fn round_trips_history() {
    let dir = temp_dir("roundtrip");
    let source = dir.join("a.txt");
    fs::write(&source, "three\n").unwrap();
    let store = UndoStore::new(dir.join("undo"));
    let history = history("three\n");
    store.save(&source, &hash("three\n"), &history).unwrap();
    let loaded = store.load(&source, &hash("three\n")).unwrap().unwrap();
    assert_eq!(loaded, history);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn hash_mismatch_discards_the_file() {
    let dir = temp_dir("hash");
    let source = dir.join("a.txt");
    fs::write(&source, "three\n").unwrap();
    let store = UndoStore::new(dir.join("undo"));
    store
        .save(&source, &hash("three\n"), &history("three\n"))
        .unwrap();
    assert!(store.load(&source, &hash("other\n")).unwrap().is_none());
    assert!(!store.file_path(&canonical_path(&source)).exists());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn corrupt_checksum_is_discarded() {
    let dir = temp_dir("checksum");
    let source = dir.join("a.txt");
    fs::write(&source, "three\n").unwrap();
    let store = UndoStore::new(dir.join("undo"));
    store
        .save(&source, &hash("three\n"), &history("three\n"))
        .unwrap();
    let file = store.file_path(&canonical_path(&source));
    let mut bytes = fs::read(&file).unwrap();
    *bytes.last_mut().unwrap() ^= 0xff;
    fs::write(&file, bytes).unwrap();
    assert!(matches!(
        store.load(&source, &hash("three\n")),
        Err(UndoStoreError::Checksum)
    ));
    assert!(!file.exists());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn wrong_version_and_truncation_are_discarded() {
    let dir = temp_dir("version");
    let source = dir.join("a.txt");
    fs::write(&source, "three\n").unwrap();
    let store = UndoStore::new(dir.join("undo"));
    store
        .save(&source, &hash("three\n"), &history("three\n"))
        .unwrap();
    let file = store.file_path(&canonical_path(&source));
    let mut bytes = fs::read(&file).unwrap();
    bytes[8..12].copy_from_slice(&2u32.to_le_bytes());
    fs::write(&file, &bytes).unwrap();
    assert!(matches!(
        store.load(&source, &hash("three\n")),
        Err(UndoStoreError::Version(2))
    ));
    assert!(!file.exists());
    store
        .save(&source, &hash("three\n"), &history("three\n"))
        .unwrap();
    let mut bytes = fs::read(&file).unwrap();
    bytes.truncate(bytes.len() - 3);
    fs::write(&file, bytes).unwrap();
    assert!(store.load(&source, &hash("three\n")).is_err());
    let _ = fs::remove_dir_all(dir);
}

#[cfg(unix)]
#[test]
fn uses_private_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let dir = temp_dir("permissions");
    let source = dir.join("a.txt");
    fs::write(&source, "three\n").unwrap();
    let store = UndoStore::new(dir.join("undo"));
    store
        .save(&source, &hash("three\n"), &history("three\n"))
        .unwrap();
    let file = store.file_path(&canonical_path(&source));
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(&store.dir).unwrap().permissions().mode() & 0o777,
        0o700
    );
    let _ = fs::remove_dir_all(dir);
}

#[cfg(unix)]
#[test]
fn fixes_existing_directory_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let dir = temp_dir("existing_permissions");
    let source = dir.join("a.txt");
    fs::write(&source, "three\n").unwrap();
    let store = UndoStore::new(dir.join("undo"));
    fs::create_dir_all(&store.dir).unwrap();
    fs::set_permissions(&store.dir, fs::Permissions::from_mode(0o755)).unwrap();
    store
        .save(&source, &hash("three\n"), &history("three\n"))
        .unwrap();
    assert_eq!(
        fs::metadata(&store.dir).unwrap().permissions().mode() & 0o777,
        0o700
    );
    let _ = fs::remove_dir_all(dir);
}

#[cfg(unix)]
#[test]
fn keys_non_utf8_paths_by_bytes() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    let dir = temp_dir("non_utf8");
    let name = OsString::from_vec(vec![b'a', 0xff, b'.', b't', b'x', b't']);
    let source = dir.join(name);
    fs::write(&source, "three\n").unwrap();
    let store = UndoStore::new(dir.join("undo"));
    store
        .save(&source, &hash("three\n"), &history("three\n"))
        .unwrap();
    assert!(store.load(&source, &hash("three\n")).unwrap().is_some());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn byte_cap_drops_old_entries_first() {
    let dir = temp_dir("cap");
    let source = dir.join("a.txt");
    fs::write(&source, "three\n").unwrap();
    let store = UndoStore::new(dir.join("undo"));
    let mut history = history("three\n");
    let original_len = history.entries.len();
    history.entries[1].delta.inserted = "x".repeat(super::format::MAX_FILE_BYTES);
    store.save(&source, &hash("three\n"), &history).unwrap();
    let loaded = store.load(&source, &hash("three\n")).unwrap().unwrap();
    assert!(loaded.entries.len() < original_len);
    assert_ne!(loaded.entries[0].id, history.entries[0].id);
    assert_eq!(loaded.current_id, history.current_id);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn last_writer_wins_with_a_valid_history() {
    let dir = temp_dir("last_writer");
    let source = dir.join("a.txt");
    fs::write(&source, "four\n").unwrap();
    let store = UndoStore::new(dir.join("undo"));
    let first = history("three\n");
    let second = history("four\n");
    store.save(&source, &hash("three\n"), &first).unwrap();
    store.save(&source, &hash("four\n"), &second).unwrap();
    assert_eq!(
        store.load(&source, &hash("four\n")).unwrap().unwrap(),
        second
    );
    assert!(store.load(&source, &hash("three\n")).unwrap().is_none());
    let _ = fs::remove_dir_all(dir);
}

#[cfg(unix)]
#[test]
fn symlinked_paths_share_a_history() {
    use std::os::unix::fs::symlink;
    let dir = temp_dir("symlink");
    let source = dir.join("a.txt");
    let link = dir.join("link.txt");
    fs::write(&source, "three\n").unwrap();
    symlink(&source, &link).unwrap();
    let store = UndoStore::new(dir.join("undo"));
    store
        .save(&link, &hash("three\n"), &history("three\n"))
        .unwrap();
    assert!(store.load(&source, &hash("three\n")).unwrap().is_some());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn oversized_undo_files_are_rejected() {
    let dir = temp_dir("oversized_file");
    let source = dir.join("a.txt");
    fs::write(&source, "three\n").unwrap();
    let store = UndoStore::new(dir.join("undo"));
    store.ensure_dir().unwrap();
    let file = store.file_path(&canonical_path(&source));
    fs::write(
        &file,
        vec![0; super::format::MAX_FILE_BYTES + MAX_PATH_BYTES as usize + 4096],
    )
    .unwrap();
    assert!(matches!(
        store.load(&source, &hash("three\n")),
        Err(UndoStoreError::TooLarge)
    ));
    assert!(!file.exists());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn load_survives_a_deleted_source_file() {
    let dir = temp_dir("deleted_source");
    let source = dir.join("a.txt");
    fs::write(&source, "three\n").unwrap();
    let store = UndoStore::new(dir.join("undo"));
    store
        .save(&source, &hash("three\n"), &history("three\n"))
        .unwrap();
    fs::remove_file(&source).unwrap();
    assert!(store.load(&source, &hash("three\n")).unwrap().is_some());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn prune_removes_files_over_the_total_cap() {
    let dir = temp_dir("prune");
    let store = UndoStore::new(dir.join("undo"));
    for name in ["a.txt", "b.txt"] {
        let source = dir.join(name);
        fs::write(&source, "three\n").unwrap();
        store
            .save(&source, &hash("three\n"), &history("three\n"))
            .unwrap();
    }
    store.prune(0, Duration::from_secs(3600)).unwrap();
    assert_eq!(fs::read_dir(&store.dir).unwrap().count(), 0);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn prune_skips_temporary_files() {
    let dir = temp_dir("prune_temp");
    let source = dir.join("a.txt");
    fs::write(&source, "three\n").unwrap();
    let store = UndoStore::new(dir.join("undo"));
    store
        .save(&source, &hash("three\n"), &history("three\n"))
        .unwrap();
    let temp = store.dir.join(".undo-live.tmp");
    fs::write(&temp, b"partial").unwrap();
    store.prune(0, Duration::from_secs(3600)).unwrap();
    assert!(temp.exists());
    assert_eq!(fs::read_dir(&store.dir).unwrap().count(), 1);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn prune_errors_do_not_panic() {
    let dir = temp_dir("prune_error");
    let store = UndoStore::new(dir.join("undo"));
    fs::write(&store.dir, b"not a directory").unwrap();
    assert!(store.prune(0, Duration::from_secs(3600)).is_err());
    let _ = fs::remove_dir_all(dir);
}

#[cfg(unix)]
#[test]
fn prune_does_not_break_open_reads() {
    use std::io::Read;
    let dir = temp_dir("prune_read");
    let source = dir.join("a.txt");
    fs::write(&source, "three\n").unwrap();
    let store = UndoStore::new(dir.join("undo"));
    store
        .save(&source, &hash("three\n"), &history("three\n"))
        .unwrap();
    let file_path = store.file_path(&canonical_path(&source));
    let mut file = File::open(&file_path).unwrap();
    store.prune(0, Duration::from_secs(3600)).unwrap();
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();
    assert!(!bytes.is_empty());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn prune_keeps_the_most_recent_files() {
    let dir = temp_dir("prune_lru");
    let first = dir.join("a.txt");
    let second = dir.join("b.txt");
    fs::write(&first, "three\n").unwrap();
    fs::write(&second, "three\n").unwrap();
    let store = UndoStore::new(dir.join("undo"));
    store
        .save(&first, &hash("three\n"), &history("three\n"))
        .unwrap();
    std::thread::sleep(Duration::from_millis(20));
    store
        .save(&second, &hash("three\n"), &history("three\n"))
        .unwrap();
    let file = store.file_path(&canonical_path(&second));
    let one_file = fs::metadata(&file).unwrap().len();
    store.prune(one_file, Duration::from_secs(3600)).unwrap();
    assert_eq!(fs::read_dir(&store.dir).unwrap().count(), 1);
    assert!(store.load(&second, &hash("three\n")).unwrap().is_some());
    assert!(store.load(&first, &hash("three\n")).unwrap().is_none());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn prune_removes_old_files() {
    let dir = temp_dir("prune_age");
    let source = dir.join("a.txt");
    fs::write(&source, "three\n").unwrap();
    let store = UndoStore::new(dir.join("undo"));
    store
        .save(&source, &hash("three\n"), &history("three\n"))
        .unwrap();
    store.prune(u64::MAX, Duration::ZERO).unwrap();
    assert_eq!(fs::read_dir(&store.dir).unwrap().count(), 0);
    let _ = fs::remove_dir_all(dir);
}
