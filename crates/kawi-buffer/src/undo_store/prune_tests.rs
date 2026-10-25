use super::test_support::{hash, history, temp_dir};
use super::*;
use std::fs;
use std::time::Duration;

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
