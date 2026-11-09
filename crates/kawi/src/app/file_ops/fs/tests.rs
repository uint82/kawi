use super::copy::{copy_tree, promote_copy};
use super::*;
use std::fs::{File, FileTimes};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, UNIX_EPOCH};

static NEXT: AtomicU64 = AtomicU64::new(1);

mod cross_device;

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
fn rename_refuses_an_existing_directory_even_with_force() {
    let dir = temp_dir("rename_dir_exists");
    let old = dir.join("old");
    let new = dir.join("new");
    fs::create_dir_all(&old).unwrap();
    fs::create_dir_all(&new).unwrap();
    fs::write(old.join("a.txt"), "a\n").unwrap();
    fs::write(new.join("keep.txt"), "keep\n").unwrap();
    assert_eq!(
        rename_path(&old, &new, true).unwrap_err().kind(),
        io::ErrorKind::AlreadyExists
    );
    assert!(old.join("a.txt").exists());
    assert!(new.join("keep.txt").exists());
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
fn no_replace_rename_moves_a_file_to_a_new_target() {
    let dir = temp_dir("no_replace_file");
    let old = dir.join("old.txt");
    let new = dir.join("new.txt");
    fs::write(&old, "old\n").unwrap();
    rename_noreplace(&old, &new).unwrap();
    assert!(!old.exists());
    assert_eq!(fs::read_to_string(&new).unwrap(), "old\n");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn no_replace_rename_refuses_an_existing_target() {
    let dir = temp_dir("no_replace_exists");
    let old = dir.join("old.txt");
    let new = dir.join("new.txt");
    fs::write(&old, "old\n").unwrap();
    fs::write(&new, "new\n").unwrap();
    assert_eq!(
        rename_noreplace(&old, &new).unwrap_err().kind(),
        io::ErrorKind::AlreadyExists
    );
    assert_eq!(fs::read_to_string(&old).unwrap(), "old\n");
    assert_eq!(fs::read_to_string(&new).unwrap(), "new\n");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn no_replace_rename_moves_a_directory_to_a_new_target() {
    let dir = temp_dir("no_replace_dir");
    let old = dir.join("old");
    let new = dir.join("new");
    fs::create_dir_all(old.join("sub")).unwrap();
    fs::write(old.join("sub/a.txt"), "alpha\n").unwrap();
    rename_noreplace(&old, &new).unwrap();
    assert!(!old.exists());
    assert_eq!(
        fs::read_to_string(new.join("sub/a.txt")).unwrap(),
        "alpha\n"
    );
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

#[cfg(unix)]
#[test]
fn failed_copy_keeps_the_source_and_removes_the_partial_target() {
    use std::os::unix::fs::PermissionsExt;

    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let source_root = temp_dir("failed_copy_source");
    let alternate = PathBuf::from("/dev/shm");
    if !alternate.is_dir() {
        let _ = fs::remove_dir_all(source_root);
        return;
    }
    let source = source_root.join("tree");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("readable.txt"), "alpha\n").unwrap();
    let blocked = source.join("blocked.txt");
    fs::write(&blocked, "bravo\n").unwrap();
    fs::set_permissions(&blocked, fs::Permissions::from_mode(0o000)).unwrap();
    let target = alternate.join(format!(
        "kawi_file_fs_failed_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&target);

    copy_tree(&source, &target).unwrap_err();
    assert!(source.join("readable.txt").exists());
    assert!(blocked.exists());
    assert!(!target.exists());
    fs::set_permissions(&blocked, fs::Permissions::from_mode(0o600)).unwrap();
    let _ = fs::remove_dir_all(source_root);
}

#[test]
fn promote_copy_never_replaces_an_existing_target_without_force() {
    let dir = temp_dir("promote_race");
    let staged = dir.join("staged.txt");
    let target = dir.join("target.txt");
    fs::write(&staged, "new\n").unwrap();
    fs::write(&target, "old\n").unwrap();
    assert_eq!(
        promote_copy(&staged, &target, false).unwrap_err().kind(),
        io::ErrorKind::AlreadyExists
    );
    assert_eq!(fs::read_to_string(&target).unwrap(), "old\n");
    assert_eq!(fs::read_to_string(&staged).unwrap(), "new\n");
    let _ = fs::remove_dir_all(dir);
}
