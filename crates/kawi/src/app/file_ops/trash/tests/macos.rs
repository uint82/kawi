use super::super::macos::{home_root, volume_root, MacOsTrash};
use super::super::topdir::user_id;
use super::super::TrashDir;
use super::temp_dir;
use std::fs;

#[test]
fn macos_trashes_a_file_without_info() {
    let dir = temp_dir("macos_file");
    let target = dir.join("a.txt");
    fs::write(&target, "alpha\n").unwrap();
    let root = dir.join("Trash");
    let trash = MacOsTrash::new(root.clone());
    let moved = trash.trash_as(&target).unwrap();
    assert!(!target.exists());
    assert_eq!(fs::read_to_string(moved).unwrap(), "alpha\n");
    assert!(!root.join("info").exists());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn macos_uses_unique_names() {
    let dir = temp_dir("macos_collision");
    let first = dir.join("first/a.txt");
    let second = dir.join("second/a.txt");
    fs::create_dir_all(first.parent().unwrap()).unwrap();
    fs::create_dir_all(second.parent().unwrap()).unwrap();
    fs::write(&first, "one\n").unwrap();
    fs::write(&second, "two\n").unwrap();
    let trash = MacOsTrash::new(dir.join("Trash"));
    let moved_first = trash.trash_as(&first).unwrap();
    let moved_second = trash.trash_as(&second).unwrap();
    assert_eq!(moved_first.file_name().unwrap(), "a.txt");
    assert_eq!(moved_second.file_name().unwrap(), "a.txt.1");
    let _ = fs::remove_dir_all(dir);
}

#[cfg(unix)]
#[test]
fn macos_volume_trash_is_private() {
    use std::os::unix::fs::PermissionsExt;

    let dir = temp_dir("macos_private");
    let target = dir.join("a.txt");
    fs::write(&target, "alpha\n").unwrap();
    let root = dir.join("Trash");
    MacOsTrash::new(root.clone()).trash_as(&target).unwrap();
    let mode = fs::metadata(&root).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o700);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn macos_home_root_uses_the_home_trash() {
    assert_eq!(
        home_root(std::path::Path::new("/Users/example")),
        std::path::PathBuf::from("/Users/example/.Trash")
    );
}

#[test]
fn macos_volume_root_prefers_shared_trashes() {
    let dir = temp_dir("macos_shared");
    fs::create_dir_all(dir.join(".Trashes")).unwrap();
    let root = volume_root(&dir).unwrap();
    assert_eq!(root, dir.join(".Trashes").join(user_id().unwrap()));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn macos_volume_root_uses_a_per_user_fallback() {
    let dir = temp_dir("macos_fallback");
    let root = volume_root(&dir).unwrap();
    assert_eq!(root, dir.join(format!(".Trash-{}", user_id().unwrap())));
    let _ = fs::remove_dir_all(dir);
}

#[cfg(unix)]
#[test]
fn macos_volume_root_ignores_symlinked_shared_trashes() {
    use std::os::unix::fs::symlink;

    let dir = temp_dir("macos_symlink");
    let outside = temp_dir("macos_symlink_outside");
    symlink(&outside, dir.join(".Trashes")).unwrap();
    let root = volume_root(&dir).unwrap();
    assert_eq!(root, dir.join(format!(".Trash-{}", user_id().unwrap())));
    let _ = fs::remove_dir_all(dir);
    let _ = fs::remove_dir_all(outside);
}

#[cfg(unix)]
#[test]
fn macos_volume_root_uses_a_fallback_for_unwritable_shared_trashes() {
    use std::os::unix::fs::PermissionsExt;

    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let dir = temp_dir("macos_unwritable");
    let shared = dir.join(".Trashes");
    fs::create_dir_all(&shared).unwrap();
    fs::set_permissions(&shared, fs::Permissions::from_mode(0o555)).unwrap();
    let root = volume_root(&dir).unwrap();
    assert_eq!(root, dir.join(format!(".Trash-{}", user_id().unwrap())));
    fs::set_permissions(&shared, fs::Permissions::from_mode(0o755)).unwrap();
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn trash_dir_delegates_to_the_macos_backend() {
    let dir = temp_dir("macos_delegate");
    let target = dir.join("a.txt");
    fs::write(&target, "alpha\n").unwrap();
    let trash = TrashDir::macos(dir.join("Trash"));
    assert!(trash.can_trash(&target));
    let moved = trash.trash_as(&target, &target).unwrap();
    assert_eq!(fs::read_to_string(moved).unwrap(), "alpha\n");
    let _ = fs::remove_dir_all(dir);
}
