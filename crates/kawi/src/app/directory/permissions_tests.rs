use std::path::PathBuf;

use super::diff::Change;
use super::entry::EntryKind;
use super::permissions::preflight;
use super::planner::Operation;
use super::test_support::FakeFs;

fn copy(from: &str, to: &str) -> Change {
    Change::Copy {
        id: 1,
        from: PathBuf::from(from),
        to: PathBuf::from(to),
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
    assert!(error.contains("Cannot modify /project/readonly"));
}

#[test]
fn rename_requires_write_access_to_both_parents() {
    let fs = FakeFs::new(&[("/project", true), ("/one", true), ("/two", true)]).deny_write("/two");
    let error = preflight(
        &[],
        &[Operation::Rename {
            from: PathBuf::from("/one/a.txt"),
            to: PathBuf::from("/two/a.txt"),
        }],
        &fs,
    )
    .unwrap_err();
    assert!(error.contains("Cannot modify /two"));
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
    assert!(error.contains("Cannot read /project/a"));
}

#[test]
fn delete_requires_delete_permission() {
    let fs = FakeFs::new(&[("/project", true), ("/project/a", false)]).deny_delete("/project/a");
    let error = preflight(&[], &[Operation::Trash(PathBuf::from("/project/a"))], &fs).unwrap_err();
    assert!(error.contains("Cannot delete /project/a"));
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
