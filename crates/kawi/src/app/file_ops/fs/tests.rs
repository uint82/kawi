use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(1);

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "kawi_file_fs_{}_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
        name
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn rename_refuses_an_existing_target_without_force() {
    let dir = temp_dir("rename_exists");
    let old = dir.join("old.txt");
    let new = dir.join("new.txt");
    fs::write(&old, "old\n").unwrap();
    fs::write(&new, "new\n").unwrap();
    assert_eq!(
        rename_path(&old, &new, false).unwrap_err().kind(),
        io::ErrorKind::AlreadyExists
    );
    rename_path(&old, &new, true).unwrap();
    assert_eq!(fs::read_to_string(&new).unwrap(), "old\n");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn rename_moves_directories_and_counts_items() {
    let dir = temp_dir("rename_dir");
    let old = dir.join("old");
    let new = dir.join("new");
    fs::create_dir_all(old.join("sub")).unwrap();
    fs::write(old.join("a.txt"), "a\n").unwrap();
    fs::write(old.join("sub/b.txt"), "b\n").unwrap();
    assert_eq!(count_tree(&old).unwrap(), 3);
    rename_path(&old, &new, false).unwrap();
    assert!(!old.exists());
    assert!(new.join("sub/b.txt").exists());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn rename_refuses_a_symlink_target_without_force() {
    let dir = temp_dir("rename_symlink");
    let old = dir.join("old.txt");
    let other = dir.join("other.txt");
    let new = dir.join("new.txt");
    fs::write(&old, "old\n").unwrap();
    fs::write(&other, "other\n").unwrap();
    std::os::unix::fs::symlink(&other, &new).unwrap();
    assert_eq!(
        rename_path(&old, &new, false).unwrap_err().kind(),
        io::ErrorKind::AlreadyExists
    );
    assert_eq!(fs::read_to_string(&old).unwrap(), "old\n");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn remove_path_removes_directories_and_symlinks() {
    let dir = temp_dir("remove");
    let nested = dir.join("nested");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join("a.txt"), "a\n").unwrap();
    remove_path(&nested, true).unwrap();
    assert!(!nested.exists());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn create_empty_file_never_overwrites() {
    let dir = temp_dir("create");
    let path = dir.join("a.txt");
    create_empty_file(&path).unwrap();
    assert_eq!(
        create_empty_file(&path).unwrap_err().kind(),
        io::ErrorKind::AlreadyExists
    );
    let _ = fs::remove_dir_all(dir);
}
