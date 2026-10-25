use super::test_support::{hash, history, temp_dir};
use super::*;
use std::fs;

#[test]
fn moves_a_history_to_a_new_path() {
    let dir = temp_dir("move_file");
    let old = dir.join("old.txt");
    let new = dir.join("new.txt");
    fs::write(&old, "two\n").unwrap();
    fs::write(&new, "two\n").unwrap();
    let store = UndoStore::new(dir.join("undo"));
    store.save(&old, &hash("two\n"), &history("two\n")).unwrap();
    store.move_prefix(&old, &new).unwrap();
    assert!(store.load(&old, &hash("two\n")).unwrap().is_none());
    assert!(store.load(&new, &hash("two\n")).unwrap().is_some());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn moves_every_history_under_a_directory_prefix() {
    let dir = temp_dir("move_tree");
    let old = dir.join("old");
    let new = dir.join("new");
    fs::create_dir_all(old.join("sub")).unwrap();
    fs::create_dir_all(&new).unwrap();
    let first = old.join("a.txt");
    let second = old.join("sub/b.txt");
    fs::write(&first, "two\n").unwrap();
    fs::write(&second, "two\n").unwrap();
    let store = UndoStore::new(dir.join("undo"));
    store
        .save(&first, &hash("two\n"), &history("two\n"))
        .unwrap();
    store
        .save(&second, &hash("two\n"), &history("two\n"))
        .unwrap();
    store.move_prefix(&old, &new).unwrap();
    assert!(store.load(&first, &hash("two\n")).unwrap().is_none());
    assert!(store.load(&second, &hash("two\n")).unwrap().is_none());
    assert!(store
        .load(&new.join("a.txt"), &hash("two\n"))
        .unwrap()
        .is_some());
    assert!(store
        .load(&new.join("sub/b.txt"), &hash("two\n"))
        .unwrap()
        .is_some());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn removes_every_history_under_a_directory_prefix() {
    let dir = temp_dir("remove_tree");
    let old = dir.join("old");
    fs::create_dir_all(old.join("sub")).unwrap();
    let first = old.join("a.txt");
    let second = old.join("sub/b.txt");
    fs::write(&first, "two\n").unwrap();
    fs::write(&second, "two\n").unwrap();
    let store = UndoStore::new(dir.join("undo"));
    store
        .save(&first, &hash("two\n"), &history("two\n"))
        .unwrap();
    store
        .save(&second, &hash("two\n"), &history("two\n"))
        .unwrap();
    store.remove_prefix(&old).unwrap();
    assert!(store.load(&first, &hash("two\n")).unwrap().is_none());
    assert!(store.load(&second, &hash("two\n")).unwrap().is_none());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn does_not_move_a_sibling_with_a_shared_name_prefix() {
    let dir = temp_dir("move_sibling");
    let old = dir.join("old");
    let new = dir.join("new");
    let sibling = dir.join("old-other");
    fs::create_dir_all(&old).unwrap();
    fs::create_dir_all(&sibling).unwrap();
    let source = old.join("a.txt");
    let sibling_source = sibling.join("b.txt");
    fs::write(&source, "two\n").unwrap();
    fs::write(&sibling_source, "two\n").unwrap();
    let store = UndoStore::new(dir.join("undo"));
    store
        .save(&source, &hash("two\n"), &history("two\n"))
        .unwrap();
    store
        .save(&sibling_source, &hash("two\n"), &history("two\n"))
        .unwrap();
    store.move_prefix(&old, &new).unwrap();
    assert!(store
        .load(&sibling_source, &hash("two\n"))
        .unwrap()
        .is_some());
    let _ = fs::remove_dir_all(dir);
}
