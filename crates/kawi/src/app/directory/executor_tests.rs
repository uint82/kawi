use std::path::PathBuf;

use super::executor::{execute, PreparedPlan};
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
            force: false,
        },
        Operation::Rename {
            from: PathBuf::from("/project/.a.tmp"),
            to: PathBuf::from("/project/b.txt"),
            force: false,
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

#[test]
fn cross_device_rename_promotes_and_defers_source_removal() {
    let mut fs = FakeFs::new(&[
        ("/project", true),
        ("/project/a.txt", false),
        ("/mnt", true),
    ])
    .cross_device("/project/a.txt", "/mnt/b.txt");
    let plan = plan(vec![Operation::Rename {
        from: PathBuf::from("/project/a.txt"),
        to: PathBuf::from("/mnt/b.txt"),
        force: false,
    }]);
    let prepared = PreparedPlan::new(&plan, &fs).unwrap();
    let stage = prepared.stage_path(0).unwrap().to_path_buf();
    let mut promotions = Vec::new();
    let outcome = prepared.execute(&mut fs, &mut |index| {
        promotions.push(index);
        Ok(())
    });
    assert!(outcome.error.is_none());
    assert_eq!(
        outcome.deferred_deletes,
        vec![PathBuf::from("/project/a.txt")]
    );
    assert_eq!(promotions, vec![0]);
    assert!(fs.exists(std::path::Path::new("/project/a.txt")));
    assert!(fs.exists(std::path::Path::new("/mnt/b.txt")));
    assert!(!fs.exists(&stage));
}

#[test]
fn cross_device_rename_rolls_back_the_promoted_target() {
    let mut fs = FakeFs::new(&[
        ("/project", true),
        ("/project/a.txt", false),
        ("/mnt", true),
    ])
    .cross_device("/project/a.txt", "/mnt/b.txt")
    .fail_at(3);
    let plan = plan(vec![
        Operation::Rename {
            from: PathBuf::from("/project/a.txt"),
            to: PathBuf::from("/mnt/b.txt"),
            force: false,
        },
        Operation::CreateFile(PathBuf::from("/project/c.txt")),
    ]);
    let outcome = execute(&plan, &mut fs);
    assert!(outcome.error.is_some());
    assert!(outcome.deferred_deletes.is_empty());
    assert!(fs.exists(std::path::Path::new("/project/a.txt")));
    assert!(!fs.exists(std::path::Path::new("/mnt/b.txt")));
}

#[test]
fn failed_promotion_record_removes_the_copy_stage() {
    let mut fs = FakeFs::new(&[
        ("/project", true),
        ("/project/a.txt", false),
        ("/mnt", true),
    ])
    .cross_device("/project/a.txt", "/mnt/b.txt");
    let plan = plan(vec![Operation::Rename {
        from: PathBuf::from("/project/a.txt"),
        to: PathBuf::from("/mnt/b.txt"),
        force: false,
    }]);
    let prepared = PreparedPlan::new(&plan, &fs).unwrap();
    let stage = prepared.stage_path(0).unwrap().to_path_buf();
    let outcome = prepared.execute(&mut fs, &mut |_| {
        Err(std::io::Error::other("journal unavailable"))
    });
    assert!(outcome.error.is_some());
    assert!(fs.exists(std::path::Path::new("/project/a.txt")));
    assert!(!fs.exists(std::path::Path::new("/mnt/b.txt")));
    assert!(!fs.exists(&stage));
}

#[test]
fn copy_uses_a_journaled_stage_and_keeps_the_source() {
    let mut fs = FakeFs::new(&[
        ("/project", true),
        ("/project/a.txt", false),
        ("/mnt", true),
    ]);
    let plan = plan(vec![Operation::Copy {
        from: PathBuf::from("/project/a.txt"),
        to: PathBuf::from("/mnt/b.txt"),
    }]);
    let prepared = PreparedPlan::new(&plan, &fs).unwrap();
    let stage = prepared.stage_path(0).unwrap().to_path_buf();
    assert!(stage
        .file_name()
        .is_some_and(|name| name.to_string_lossy().contains(".kawi-copy-")));
    let outcome = prepared.execute(&mut fs, &mut |_| Ok(()));
    assert!(outcome.error.is_none());
    assert!(outcome.deferred_deletes.is_empty());
    assert!(fs.exists(std::path::Path::new("/project/a.txt")));
    assert!(fs.exists(std::path::Path::new("/mnt/b.txt")));
    assert!(!fs.exists(&stage));
}

#[test]
fn failed_copy_promotion_removes_the_stage_and_target() {
    let mut fs = FakeFs::new(&[
        ("/project", true),
        ("/project/a.txt", false),
        ("/mnt", true),
    ])
    .fail_at(1);
    let plan = plan(vec![Operation::Copy {
        from: PathBuf::from("/project/a.txt"),
        to: PathBuf::from("/mnt/b.txt"),
    }]);
    let prepared = PreparedPlan::new(&plan, &fs).unwrap();
    let stage = prepared.stage_path(0).unwrap().to_path_buf();
    let outcome = prepared.execute(&mut fs, &mut |_| Ok(()));
    assert!(outcome.error.is_some());
    assert!(fs.exists(std::path::Path::new("/project/a.txt")));
    assert!(!fs.exists(std::path::Path::new("/mnt/b.txt")));
    assert!(!fs.exists(&stage));
}
