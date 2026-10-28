use std::path::PathBuf;

use super::diff::Change;
use super::entry::EntryKind;
use super::executor::execute;
use super::fs::DirectoryFs;
use super::planner::{plan, Operation};
use super::test_support::FakeFs;

fn rename(id: u64, from: &str, to: &str) -> Change {
    Change::Rename {
        id,
        from: PathBuf::from(from),
        to: PathBuf::from(to),
        kind: EntryKind::File,
    }
}

fn create(path: &str, kind: EntryKind) -> Change {
    Change::Create {
        path: PathBuf::from(path),
        kind,
    }
}

#[test]
fn swaps_use_temporary_names() {
    let fs = FakeFs::new(&[("/project/a", false), ("/project/b", false)]);
    let plan = plan(
        vec![
            rename(1, "/project/a", "/project/b"),
            rename(2, "/project/b", "/project/a"),
        ],
        &fs,
    )
    .unwrap();
    let renames = plan
        .operations
        .iter()
        .filter(|operation| matches!(operation, Operation::Rename { .. }))
        .count();
    assert_eq!(renames, 4);
    let mut fs = fs;
    let outcome = execute(&plan, &mut fs);
    assert!(outcome.error.is_none());
    assert!(fs.exists(std::path::Path::new("/project/a")));
    assert!(fs.exists(std::path::Path::new("/project/b")));
}

#[test]
fn parent_directories_are_created_before_files() {
    let fs = FakeFs::new(&[("/project", true)]);
    let plan = plan(vec![create("/project/a/b/c.txt", EntryKind::File)], &fs).unwrap();
    let paths: Vec<&Operation> = plan.operations.iter().collect();
    assert!(matches!(paths[0], Operation::CreateDirectory(path) if path.ends_with("a")));
    assert!(matches!(paths[1], Operation::CreateDirectory(path) if path.ends_with("b")));
    assert!(matches!(paths[2], Operation::CreateFile(path) if path.ends_with("c.txt")));
}

#[test]
fn rename_targets_must_not_exist_unless_vacated() {
    let fs = FakeFs::new(&[("/project/a", false), ("/project/b", false)]);
    let error = plan(vec![rename(1, "/project/a", "/project/b")], &fs).unwrap_err();
    assert!(error.contains("already exists"));
}

#[test]
fn directories_cannot_move_into_themselves() {
    let fs = FakeFs::new(&[("/project/a", true)]);
    let change = Change::Rename {
        id: 1,
        from: PathBuf::from("/project/a"),
        to: PathBuf::from("/project/a/b"),
        kind: EntryKind::Directory,
    };
    let error = plan(vec![change], &fs).unwrap_err();
    assert!(error.contains("into itself"));
}

#[test]
fn copies_use_the_renamed_primary_target() {
    let fs = FakeFs::new(&[("/project/a", false)]);
    let plan = plan(
        vec![
            rename(1, "/project/a", "/project/b"),
            Change::Copy {
                id: 1,
                from: PathBuf::from("/project/a"),
                to: PathBuf::from("/project/c"),
                kind: EntryKind::File,
            },
        ],
        &fs,
    )
    .unwrap();
    let copy = plan
        .operations
        .iter()
        .find(|operation| matches!(operation, Operation::Copy { .. }))
        .unwrap();
    assert!(
        matches!(copy, Operation::Copy { from, to } if from == &PathBuf::from("/project/b") && to == &PathBuf::from("/project/c"))
    );
}
