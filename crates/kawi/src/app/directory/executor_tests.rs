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
fn executor_stops_at_the_first_failure() {
    let mut fs = FakeFs::new(&[("/project", true)]).fail_at(1);
    let plan = plan(vec![
        Operation::CreateDirectory(PathBuf::from("/project/sub")),
        Operation::CreateFile(PathBuf::from("/project/sub/a.txt")),
    ]);
    let outcome = execute(&plan, &mut fs);
    assert_eq!(outcome.applied, 1);
    assert!(outcome.error.is_some());
    assert!(fs.exists(std::path::Path::new("/project/sub")));
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
    assert_eq!(outcome.applied, 3);
    assert_eq!(
        fs.calls,
        vec![
            Call::Rename(
                PathBuf::from("/project/a.txt"),
                PathBuf::from("/project/.a.tmp")
            ),
            Call::Rename(
                PathBuf::from("/project/.a.tmp"),
                PathBuf::from("/project/b.txt")
            ),
            Call::Trash(PathBuf::from("/project/b.txt")),
        ]
    );
}
