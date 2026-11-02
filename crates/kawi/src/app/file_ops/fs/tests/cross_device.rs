use super::*;

#[cfg(unix)]
#[test]
fn cross_filesystem_rename_preserves_metadata_and_symlinks() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let source_root = temp_dir("exdev_source");
    let alternate = PathBuf::from("/dev/shm");
    if !alternate.is_dir()
        || fs::metadata(&source_root).unwrap().dev() == fs::metadata(&alternate).unwrap().dev()
    {
        let _ = fs::remove_dir_all(source_root);
        return;
    }
    let destination_root = alternate.join(format!(
        "kawi_file_fs_exdev_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&destination_root);
    fs::create_dir_all(&destination_root).unwrap();
    let old = source_root.join("tree");
    let new = destination_root.join("tree");
    fs::create_dir_all(&old).unwrap();
    let file = old.join("file.txt");
    fs::write(&file, "alpha\n").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o640)).unwrap();
    let stamp = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    File::options()
        .write(true)
        .open(&file)
        .unwrap()
        .set_times(FileTimes::new().set_modified(stamp))
        .unwrap();
    fs::set_permissions(&old, fs::Permissions::from_mode(0o750)).unwrap();
    std::os::unix::fs::symlink("file.txt", old.join("link.txt")).unwrap();
    let link_time = fs::symlink_metadata(old.join("link.txt"))
        .unwrap()
        .modified()
        .unwrap();

    let outcome = rename_path(&old, &new, false).unwrap();
    assert_eq!(outcome.kind, RenameKind::Copied);
    assert!(!old.exists());
    assert_eq!(fs::read_to_string(new.join("file.txt")).unwrap(), "alpha\n");
    assert_eq!(
        fs::metadata(new.join("file.txt"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o640
    );
    assert_eq!(
        fs::metadata(new.join("file.txt"))
            .unwrap()
            .modified()
            .unwrap(),
        stamp
    );
    assert_eq!(
        fs::metadata(&new).unwrap().permissions().mode() & 0o777,
        0o750
    );
    assert_eq!(
        fs::read_link(new.join("link.txt")).unwrap(),
        PathBuf::from("file.txt")
    );
    assert_eq!(
        fs::symlink_metadata(new.join("link.txt"))
            .unwrap()
            .modified()
            .unwrap(),
        link_time
    );
    let _ = fs::remove_dir_all(source_root);
    let _ = fs::remove_dir_all(destination_root);
}

#[cfg(unix)]
#[test]
fn cross_filesystem_force_replaces_the_target_atomically() {
    use std::os::unix::fs::MetadataExt;

    let source_root = temp_dir("exdev_force_source");
    let alternate = PathBuf::from("/dev/shm");
    if !alternate.is_dir()
        || fs::metadata(&source_root).unwrap().dev() == fs::metadata(&alternate).unwrap().dev()
    {
        let _ = fs::remove_dir_all(source_root);
        return;
    }
    let destination_root = alternate.join(format!(
        "kawi_file_fs_force_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&destination_root);
    fs::create_dir_all(&destination_root).unwrap();
    let old = source_root.join("old.txt");
    let new = destination_root.join("new.txt");
    fs::write(&old, "new\n").unwrap();
    fs::write(&new, "old\n").unwrap();

    let outcome = rename_path(&old, &new, true).unwrap();
    assert_eq!(outcome.kind, RenameKind::Copied);
    assert!(!old.exists());
    assert_eq!(fs::read_to_string(&new).unwrap(), "new\n");
    let staged = fs::read_dir(&destination_root)
        .unwrap()
        .filter_map(Result::ok)
        .any(|entry| entry.file_name().to_string_lossy().contains(".kawi-copy-"));
    assert!(!staged);
    let _ = fs::remove_dir_all(source_root);
    let _ = fs::remove_dir_all(destination_root);
}

#[cfg(unix)]
#[test]
fn failed_cross_filesystem_copy_keeps_source_and_target() {
    use std::os::unix::fs::MetadataExt;
    use std::os::unix::fs::PermissionsExt;

    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let source_root = temp_dir("exdev_failed_source");
    let alternate = PathBuf::from("/dev/shm");
    if !alternate.is_dir()
        || fs::metadata(&source_root).unwrap().dev() == fs::metadata(&alternate).unwrap().dev()
    {
        let _ = fs::remove_dir_all(source_root);
        return;
    }
    let destination_root = alternate.join(format!(
        "kawi_file_fs_failed_target_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&destination_root);
    fs::create_dir_all(&destination_root).unwrap();
    let source = source_root.join("tree");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "alpha\n").unwrap();
    let blocked = source.join("blocked.txt");
    fs::write(&blocked, "bravo\n").unwrap();
    fs::set_permissions(&blocked, fs::Permissions::from_mode(0o000)).unwrap();
    let target = destination_root.join("target.txt");
    fs::write(&target, "keep\n").unwrap();

    assert!(rename_path(&source, &target, true).is_err());
    assert!(source.join("a.txt").exists());
    assert_eq!(fs::read_to_string(&target).unwrap(), "keep\n");
    let staged = fs::read_dir(&destination_root)
        .unwrap()
        .filter_map(Result::ok)
        .any(|entry| entry.file_name().to_string_lossy().contains(".kawi-copy-"));
    assert!(!staged);
    fs::set_permissions(&blocked, fs::Permissions::from_mode(0o600)).unwrap();
    let _ = fs::remove_dir_all(source_root);
    let _ = fs::remove_dir_all(destination_root);
}

#[cfg(unix)]
#[test]
fn staged_cross_filesystem_move_keeps_the_source_until_cleanup() {
    use std::os::unix::fs::MetadataExt;

    let source_root = temp_dir("staged_exdev_source");
    let alternate = PathBuf::from("/dev/shm");
    if !alternate.is_dir()
        || fs::metadata(&source_root).unwrap().dev() == fs::metadata(&alternate).unwrap().dev()
    {
        let _ = fs::remove_dir_all(source_root);
        return;
    }
    let destination_root = alternate.join(format!(
        "kawi_file_fs_staged_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&destination_root);
    fs::create_dir_all(&destination_root).unwrap();
    let old = source_root.join("a.txt");
    let new = destination_root.join("b.txt");
    let stage = destination_root.join(".b.txt.kawi-copy-0-0");
    fs::write(&old, "alpha\n").unwrap();

    assert_eq!(
        try_rename_path(&old, &new, false).unwrap_err().kind(),
        io::ErrorKind::CrossesDevices
    );
    stage_copy(&old, &stage).unwrap();
    assert!(old.exists());
    assert!(stage.exists());
    promote_stage(&stage, &new, false).unwrap();
    assert!(old.exists());
    assert!(!stage.exists());
    assert_eq!(fs::read_to_string(&new).unwrap(), "alpha\n");
    remove_path(&old, true).unwrap();
    assert!(!old.exists());
    let _ = fs::remove_dir_all(source_root);
    let _ = fs::remove_dir_all(destination_root);
}
