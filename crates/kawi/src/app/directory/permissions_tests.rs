use std::path::PathBuf;

use super::diff::Change;
use super::entry::EntryKind;
#[cfg(target_os = "linux")]
use super::fs::DirectoryFs;
use super::fs::RealFs;
use super::permissions::preflight;
use super::planner::{plan as build_plan, Operation};
use super::test_support::FakeFs;

fn copy(from: &str, to: &str) -> Change {
    Change::Copy {
        id: 1,
        from: PathBuf::from(from),
        to: PathBuf::from(to),
        kind: EntryKind::File,
    }
}

fn delete(path: &str) -> Change {
    Change::Delete {
        id: 1,
        path: PathBuf::from(path),
        kind: EntryKind::File,
    }
}

#[test]
fn write_permission_checks_the_nearest_existing_parent() {
    let fs = FakeFs::new(&[("/project", true), ("/project/readonly", true)])
        .deny_write("/project/readonly");
    let error = preflight(
        &[],
        &[Operation::CreateFile(PathBuf::from(
            "/project/readonly/missing/a.txt",
        ))],
        &fs,
    )
    .unwrap_err();
    assert!(error.contains("create /project/readonly/missing/a.txt"));
    assert!(error.contains("parent /project/readonly"));
    assert!(error.contains("write and execute permission"));
}

#[test]
fn rename_requires_write_access_to_both_parents() {
    let fs = FakeFs::new(&[("/project", true), ("/one", true), ("/two", true)]).deny_write("/two");
    let error = preflight(
        &[],
        &[Operation::Rename {
            from: PathBuf::from("/one/a.txt"),
            to: PathBuf::from("/two/a.txt"),
            force: false,
        }],
        &fs,
    )
    .unwrap_err();
    assert!(error.contains("move /one/a.txt to /two/a.txt"));
    assert!(error.contains("parent /two"));
    assert!(error.contains("write and execute permission"));
}

#[test]
fn cross_parent_directory_move_requires_write_on_the_directory() {
    let fs = FakeFs::new(&[("/one", true), ("/two", true), ("/one/dir", true)])
        .deny_directory_write("/one/dir");
    let error = preflight(
        &[Change::Rename {
            id: 1,
            from: PathBuf::from("/one/dir"),
            to: PathBuf::from("/two/dir"),
            kind: EntryKind::Directory,
        }],
        &[],
        &fs,
    )
    .unwrap_err();
    assert!(error.contains("move /one/dir to /two/dir"));
    assert!(error.contains("directory /one/dir"));
    assert!(error.contains("write permission"));
}

#[test]
fn copy_requires_a_readable_source() {
    let fs = FakeFs::new(&[("/project", true), ("/project/a", false)]).deny_read("/project/a");
    let error = preflight(
        &[copy("/project/a", "/project/b")],
        &[Operation::Copy {
            from: PathBuf::from("/project/renamed"),
            to: PathBuf::from("/project/b"),
        }],
        &fs,
    )
    .unwrap_err();
    assert!(error.contains("copy /project/a to /project/b"));
    assert!(error.contains("source /project/a"));
    assert!(error.contains("read permission"));
}

#[test]
fn delete_reports_sticky_and_trash_failures() {
    let fs = FakeFs::new(&[("/tmp", true), ("/tmp/victim", false)])
        .deny_entry_remove("/tmp/victim")
        .deny_delete("/tmp/victim");
    let error = preflight(
        &[delete("/tmp/victim")],
        &[Operation::Trash(PathBuf::from("/tmp/victim"))],
        &fs,
    )
    .unwrap_err();
    assert!(error.contains("delete /tmp/victim"));
    assert!(error.contains("sticky directory /tmp"));
    assert!(error.contains("no trash available"));
}

#[test]
fn all_failures_are_reported() {
    let fs = FakeFs::new(&[("/project", true), ("/one", true), ("/two", true)])
        .deny_write("/one")
        .deny_write("/two");
    let error = preflight(
        &[],
        &[
            Operation::CreateFile(PathBuf::from("/one/a.txt")),
            Operation::CreateFile(PathBuf::from("/two/b.txt")),
        ],
        &fs,
    )
    .unwrap_err();
    assert!(error.contains("create /one/a.txt"));
    assert!(error.contains("parent /one"));
    assert!(error.contains("create /two/b.txt"));
    assert!(error.contains("parent /two"));
}

#[test]
fn allowed_requirements_pass() {
    let fs = FakeFs::new(&[("/project", true)]);
    preflight(
        &[],
        &[Operation::CreateFile(PathBuf::from("/project/a.txt"))],
        &fs,
    )
    .unwrap();
}

#[cfg(unix)]
#[test]
fn cross_parent_directory_move_requires_write_on_the_source_directory() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use crate::test_support::temp_dir;

    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let root = temp_dir("permission_directory_move");
    let first = root.join("first");
    let second = root.join("second");
    let source = first.join("dir");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&second).unwrap();
    fs::set_permissions(&source, fs::Permissions::from_mode(0o555)).unwrap();
    let move_directory = |to: PathBuf| Change::Rename {
        id: 1,
        from: source.clone(),
        to,
        kind: EntryKind::Directory,
    };
    let error =
        build_plan(vec![move_directory(second.join("dir"))], &RealFs::new(true)).unwrap_err();
    assert!(error.contains("write permission"));
    assert!(error.contains(&source.to_string_lossy().into_owned()));
    build_plan(
        vec![move_directory(first.join("renamed"))],
        &RealFs::new(true),
    )
    .unwrap();
    fs::set_permissions(&source, fs::Permissions::from_mode(0o755)).unwrap();
    let _ = fs::remove_dir_all(root);
}

#[cfg(target_os = "linux")]
#[test]
fn delete_error_reports_unavailable_trash_on_another_filesystem() {
    let error = RealFs::new(false)
        .delete_error(std::path::Path::new("/proc/version"))
        .unwrap();
    assert!(error.contains("no writable trash available"));
    assert!(error.contains("directory_trash=false"));
}
