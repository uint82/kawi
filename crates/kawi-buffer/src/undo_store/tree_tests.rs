use super::test_support::{hash, temp_dir};
use super::*;
use crate::history::History;
use ropey::Rope;
use std::fs;

#[test]
fn byte_cap_trims_a_tree_without_orphans() {
    let dir = temp_dir("tree_cap");
    let source = dir.join("a.txt");
    fs::write(&source, "four\n").unwrap();
    let store = UndoStore::new(dir.join("undo"));
    let mut history = History::new();
    history.checkpoint(&Rope::from_str("one\n"), 0, 0, 0);
    history.checkpoint(&Rope::from_str("two\n"), 0, 0, 1);
    history.checkpoint(&Rope::from_str("three\n"), 0, 0, 2);
    let mut text = Rope::from_str("three\n");
    history.undo(&mut text, 0, 0, 0).unwrap();
    history.checkpoint(&text, 0, 0, 0);
    let current = Rope::from_str("four\n");
    let mut persisted = history.export(&current, 0, 0, 3);
    let current_entry = persisted
        .entries
        .iter_mut()
        .find(|entry| entry.id == persisted.current_id)
        .unwrap();
    current_entry.delta.inserted = "x".repeat(super::format::MAX_FILE_BYTES);
    store.save(&source, &hash("four\n"), &persisted).unwrap();
    let file = store.file_path(&canonical_path(&source));
    assert!(fs::metadata(&file).unwrap().len() <= super::format::MAX_FILE_BYTES as u64);
    let loaded = store.load(&source, &hash("four\n")).unwrap().unwrap();
    let ids: std::collections::HashSet<u64> = loaded.entries.iter().map(|entry| entry.id).collect();
    assert!(ids.contains(&loaded.current_id));
    let mut roots = 0;
    for entry in &loaded.entries {
        match entry.parent {
            Some(parent) => assert!(ids.contains(&parent)),
            None => roots += 1,
        }
        if let Some(child) = entry.last_visited_child {
            assert_eq!(
                loaded
                    .entries
                    .iter()
                    .find(|candidate| candidate.id == child)
                    .unwrap()
                    .parent,
                Some(entry.id)
            );
        }
    }
    assert_eq!(roots, 1);
    assert!(History::from_persisted(&current, &loaded).is_some());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn loads_a_version_one_history() {
    let dir = temp_dir("version_one");
    let source = dir.join("a.txt");
    fs::write(&source, "two\n").unwrap();
    let store = UndoStore::new(dir.join("undo"));
    store.ensure_dir().unwrap();
    let file = store.file_path(&canonical_path(&source));
    let path = super::format::path_bytes(&canonical_path(&source));
    let mut payload = Vec::new();
    payload.extend_from_slice(&1u64.to_le_bytes());
    payload.extend_from_slice(&2u32.to_le_bytes());
    for (id, parent, removed, inserted) in [
        (0u64, u64::MAX, "", "one\n"),
        (1u64, 0u64, "one\n", "two\n"),
    ] {
        payload.extend_from_slice(&id.to_le_bytes());
        payload.extend_from_slice(&parent.to_le_bytes());
        payload.extend_from_slice(&0u64.to_le_bytes());
        payload.extend_from_slice(&0u64.to_le_bytes());
        payload.extend_from_slice(&0u64.to_le_bytes());
        payload.extend_from_slice(&0u64.to_le_bytes());
        for text in [removed, inserted] {
            payload.extend_from_slice(&(text.len() as u32).to_le_bytes());
            payload.extend_from_slice(text.as_bytes());
        }
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(super::format::MAGIC);
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&hash("two\n"));
    bytes.extend_from_slice(&(path.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&path);
    bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&Sha256::digest(&payload));
    bytes.extend_from_slice(&payload);
    fs::write(&file, bytes).unwrap();
    let loaded = store.load(&source, &hash("two\n")).unwrap().unwrap();
    assert_eq!(loaded.current_id, 1);
    assert_eq!(loaded.clean_id, Some(1));
    assert_eq!(loaded.entries.len(), 2);
    let mut restored = History::from_persisted(&Rope::from_str("two\n"), &loaded).expect("restore");
    let mut text = Rope::from_str("two\n");
    assert!(restored.undo(&mut text, 0, 0, 0).is_some());
    assert_eq!(text.to_string(), "one\n");
    let _ = fs::remove_dir_all(dir);
}
