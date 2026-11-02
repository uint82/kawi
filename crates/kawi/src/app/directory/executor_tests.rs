use std::path::PathBuf;

use super::executor::execute;
use super::fs::DirectoryFs;
use super::planner::{Operation, Plan};
use super::test_support::{Call, FakeFs};

fn plan(operations: Vec<Operation>) -> Plan {
    Plan {
        operations,
        changes: 1,
    }
}

#[test]
fn executor_rolls_back_the_first_failure() {
    let mut fs = FakeFs::new(&[("/project", true)]).fail_at(1);
    let plan = plan(vec![
        Operation::CreateDirectory(PathBuf::from("/project/sub")),
        Operation::CreateFile(PathBuf::from("/project/sub/a.txt")),
    ]);
    let outcome = execute(&plan, &mut fs);
    assert_eq!(outcome.completed, 1);
    assert!(outcome.remaining.is_empty());
    assert!(outcome.rollback_errors.is_empty());
    assert!(outcome.error.is_some());
    assert!(!fs.exists(std::path::Path::new("/project/sub")));
    assert!(!fs.exists(std::path::Path::new("/project/sub/a.txt")));
}

#[test]
fn executor_reports_each_operation_kind() {
    let mut fs = FakeFs::new(&[("/project/a.txt", false)]);
    let plan = plan(vec![
        Operation::Rename {
            from: PathBuf::from("/project/a.txt"),
            to: PathBuf::from("/project/.a.tmp"),
        },
        Operation::Rename {
            from: PathBuf::from("/project/.a.tmp"),
            to: PathBuf::from("/project/b.txt"),
        },
        Operation::Trash(PathBuf::from("/project/b.txt")),
    ]);
    let outcome = execute(&plan, &mut fs);
    assert!(outcome.error.is_none());
    assert_eq!(outcome.completed, 3);
    assert_eq!(outcome.remaining, vec![0, 1, 2]);
    assert_eq!(
        fs.calls.first(),
        Some(&Call::Rename(
            PathBuf::from("/project/a.txt"),
            PathBuf::from("/project/.a.tmp")
        ))
    );
    assert!(matches!(
        fs.calls.last(),
        Some(Call::Trash(path)) if path == std::path::Path::new("/project/b.txt")
    ));
}

#[test]
fn executor_restores_staged_deletes_after_finalize_failures() {
    let mut fs = FakeFs::new(&[("/project/a.txt", false)]).fail_at(2);
    let plan = plan(vec![
        Operation::Trash(PathBuf::from("/project/a.txt")),
        Operation::CreateFile(PathBuf::from("/project/b.txt")),
    ]);
    let outcome = execute(&plan, &mut fs);
    assert!(outcome.error.is_some());
    assert!(outcome.remaining.is_empty());
    assert!(outcome.rollback_errors.is_empty());
    assert!(fs.exists(std::path::Path::new("/project/a.txt")));
    assert!(!fs.exists(std::path::Path::new("/project/b.txt")));
}

#[test]
fn executor_keeps_finalized_deletes_after_later_delete_failures() {
    let mut fs = FakeFs::new(&[("/project/a.txt", false), ("/project/b.txt", false)]).fail_at(4);
    let plan = plan(vec![
        Operation::Trash(PathBuf::from("/project/a.txt")),
        Operation::Trash(PathBuf::from("/project/b.txt")),
        Operation::CreateFile(PathBuf::from("/project/c.txt")),
    ]);
    let outcome = execute(&plan, &mut fs);
    assert!(outcome.error.is_some());
    assert_eq!(outcome.remaining, vec![0]);
    assert!(outcome.rollback_errors.is_empty());
    assert!(!fs.exists(std::path::Path::new("/project/a.txt")));
    assert!(fs.exists(std::path::Path::new("/project/b.txt")));
    assert!(!fs.exists(std::path::Path::new("/project/c.txt")));
}
