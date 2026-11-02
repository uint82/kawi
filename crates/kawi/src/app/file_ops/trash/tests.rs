use super::topdir::user_id;
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(1);

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "kawi_trash_{}_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
        name
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn trashes_a_file_and_writes_its_info() {
    let dir = temp_dir("file");
    let target = dir.join("a b.txt");
    fs::write(&target, "alpha\n").unwrap();
    let trash = TrashDir::new(dir.join("Trash"));
    let moved = trash.trash_as(&target, &target).unwrap();
    assert!(!target.exists());
    assert_eq!(fs::read_to_string(&moved).unwrap(), "alpha\n");
    let info = fs::read_to_string(trash.info_dir().join("a b.txt.trashinfo")).unwrap();
    assert!(info.contains("Path="));
    assert!(info.contains("a%20b.txt"));
    assert!(info.contains("DeletionDate="));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn trashes_directories_and_uses_a_unique_name() {
    let dir = temp_dir("directory");
    let first = dir.join("first");
    let second = dir.join("second");
    fs::create_dir_all(&first).unwrap();
    fs::create_dir_all(&second).unwrap();
    fs::write(first.join("a.txt"), "a\n").unwrap();
    let trash = TrashDir::new(dir.join("Trash"));
    let moved_first = trash.trash_as(&first, &first).unwrap();
    let moved_second = trash.trash_as(&second, &second).unwrap();
    assert_eq!(moved_first.file_name().unwrap(), "first");
    assert_eq!(moved_second.file_name().unwrap(), "second");
    assert!(moved_first.join("a.txt").exists());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn trashes_a_staged_file_under_its_original_name() {
    let dir = temp_dir("staged");
    let original = dir.join("a b.txt");
    let staged = dir.join(".a b.txt.kawi-delete-0-0");
    fs::write(&staged, "alpha\n").unwrap();
    let trash = TrashDir::new(dir.join("Trash"));
    let moved = trash.trash_as(&staged, &original).unwrap();
    assert!(!staged.exists());
    assert_eq!(moved.file_name().unwrap(), "a b.txt");
    let info = fs::read_to_string(trash.info_dir().join("a b.txt.trashinfo")).unwrap();
    assert!(info.contains("a%20b.txt"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn refuses_to_trash_across_filesystems() {
    let dir = temp_dir("devices");
    let target = dir.join("a.txt");
    fs::write(&target, "alpha\n").unwrap();
    let trash = TrashDir::new(dir.join("Trash"));
    assert!(trash.can_trash(&target));
    let _ = fs::remove_dir_all(dir);
}

#[cfg(unix)]
#[test]
fn an_unwritable_trash_location_is_unavailable() {
    use std::os::unix::fs::PermissionsExt;

    let dir = temp_dir("unwritable");
    let target = dir.join("a.txt");
    fs::write(&target, "alpha\n").unwrap();
    let trash = TrashDir::new(dir.join("Trash"));
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();
    assert!(!trash.can_trash(&target));
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn topdir_trash_uses_a_relative_info_path() {
    let dir = temp_dir("topdir");
    let topdir = fs::canonicalize(&dir).unwrap();
    let target = topdir.join("sub/a.txt");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(&target, "alpha\n").unwrap();
    let root = topdir.join(format!(".Trash-{}", user_id().unwrap()));
    let trash = TrashDir {
        root,
        topdir: Some(topdir.clone()),
    };
    let info = trash.info_path(&target);
    assert_eq!(info, "sub/a.txt");
    let _ = fs::remove_dir_all(dir);
}

#[cfg(target_os = "linux")]
#[test]
fn topdir_trash_uses_the_target_filesystem_root() {
    let trash = topdir_trash(std::path::Path::new("/dev/shm")).unwrap();
    assert!(trash.root.starts_with("/dev/shm"));
}

#[test]
fn formats_a_stable_deletion_date() {
    assert_eq!(civil_from_days(0), (1970, 1, 1));
    assert_eq!(civil_from_days(19_723), (2024, 1, 1));
    let date = deletion_date();
    assert_eq!(date.len(), 19);
    assert_eq!(&date[4..5], "-");
    assert_eq!(&date[10..11], "T");
}
