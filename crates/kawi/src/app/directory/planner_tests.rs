use std::path::PathBuf;

use super::diff::Change;
use super::entry::EntryKind;
use super::executor::execute;
use super::fs::DirectoryFs;
use super::planner::{plan, Operation};
use super::test_support::{Call, FakeFs};

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
    let fs = FakeFs::new(&[
        ("/project", true),
        ("/project/a", false),
        ("/project/b", false),
    ]);
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
fn cycles_use_temporary_names() {
    let fs = FakeFs::new(&[
        ("/project", true),
        ("/project/a", false),
        ("/project/b", false),
        ("/project/c", false),
    ]);
    let plan = plan(
        vec![
            rename(1, "/project/a", "/project/b"),
            rename(2, "/project/b", "/project/c"),
            rename(3, "/project/c", "/project/a"),
        ],
        &fs,
    )
    .unwrap();
    let renames = plan
        .operations
        .iter()
        .filter(|operation| matches!(operation, Operation::Rename { .. }))
        .count();
    assert_eq!(renames, 6);
    let mut fs = fs;
    let outcome = execute(&plan, &mut fs);
    assert!(outcome.error.is_none());
    for path in ["/project/a", "/project/b", "/project/c"] {
        assert!(fs.exists(std::path::Path::new(path)));
    }
}

#[test]
fn nested_sources_move_before_their_parent_directory() {
    let fs = FakeFs::new(&[
        ("/project", true),
        ("/project/dir", true),
        ("/project/dir/file", false),
    ]);
    let plan = plan(
        vec![
            Change::Rename {
                id: 1,
                from: PathBuf::from("/project/dir"),
                to: PathBuf::from("/project/renamed"),
                kind: EntryKind::Directory,
            },
            rename(2, "/project/dir/file", "/project/file"),
        ],
        &fs,
    )
    .unwrap();
    let mut fs = fs;
    let outcome = execute(&plan, &mut fs);
    assert!(outcome.error.is_none());
    assert!(fs.exists(std::path::Path::new("/project/renamed")));
    assert!(fs.exists(std::path::Path::new("/project/file")));
    assert!(matches!(
        fs.calls.first(),
        Some(Call::Rename(_, to)) if to.parent() == Some(std::path::Path::new("/project"))
    ));
}

#[test]
fn moves_into_renamed_directories_are_refused() {
    for target in ["/project/dir/file", "/project/renamed/file"] {
        let fs = FakeFs::new(&[("/project/dir", true), ("/project/item", false)]);
        let error = plan(
            vec![
                Change::Rename {
                    id: 1,
                    from: PathBuf::from("/project/dir"),
                    to: PathBuf::from("/project/renamed"),
                    kind: EntryKind::Directory,
                },
                rename(2, "/project/item", target),
            ],
            &fs,
        )
        .unwrap_err();
        assert!(error.contains("directory being renamed"));
        assert!(error.contains("/project/dir"));
        assert!(error.contains("/project/renamed"));
    }
}

#[test]
fn unmatched_deletes_are_trashed() {
    let fs = FakeFs::new(&[("/project", true), ("/project/a", false)]);
    let plan = plan(
        vec![Change::Delete {
            id: 1,
            path: PathBuf::from("/project/a"),
            kind: EntryKind::File,
        }],
        &fs,
    )
    .unwrap();
    assert!(matches!(
        plan.operations.as_slice(),
        [Operation::Trash(path)] if path == std::path::Path::new("/project/a")
    ));
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
    let fs = FakeFs::new(&[
        ("/project", true),
        ("/project/a", false),
        ("/project/b", false),
    ]);
    let error = plan(vec![rename(1, "/project/a", "/project/b")], &fs).unwrap_err();
    assert!(error.contains("already exists"));
}

#[test]
fn copies_follow_sources_when_ids_repeat_across_directories() {
    let fs = FakeFs::new(&[
        ("/one", true),
        ("/one/a", false),
        ("/two", true),
        ("/two/c", false),
    ]);
    let plan = plan(
        vec![
            rename(1, "/one/a", "/one/b"),
            Change::Copy {
                id: 1,
                from: PathBuf::from("/two/c"),
                to: PathBuf::from("/two/d"),
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
    assert!(matches!(
        copy,
        Operation::Copy { from, .. } if from == std::path::Path::new("/two/c")
    ));
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
    let fs = FakeFs::new(&[("/project", true), ("/project/a", false)]);
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
