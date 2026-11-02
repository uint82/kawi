use super::*;

fn journal() -> ApplyJournal {
    ApplyJournal::prepared(vec![
        JournalOperation::Rename {
            from: JournalPath::from(PathBuf::from("/project/a.txt")),
            to: JournalPath::from(PathBuf::from("/project/.a.tmp")),
            stage: None,
            force: false,
        },
        JournalOperation::Delete {
            path: JournalPath::from(PathBuf::from("/project/b.txt")),
            staged: JournalPath::from(PathBuf::from("/project/.b.tmp")),
        },
    ])
}

#[test]
fn journals_round_trip() {
    let journal = journal();
    let decoded = ApplyJournal::from_bytes(&journal.to_bytes().unwrap()).unwrap();
    assert_eq!(decoded, journal);
    assert!(decoded.is_prepared());
    assert!(!decoded.committed(Vec::new()).is_prepared());
}

#[test]
fn stages_and_deferred_sources_round_trip() {
    let journal = ApplyJournal::prepared(vec![JournalOperation::Rename {
        from: JournalPath::from(PathBuf::from("/project/a.txt")),
        to: JournalPath::from(PathBuf::from("/mnt/b.txt")),
        stage: Some(JournalPath::from(PathBuf::from(
            "/mnt/.b.txt.kawi-copy-0-0",
        ))),
        force: false,
    }])
    .committed(vec![JournalCleanup::remove(PathBuf::from(
        "/project/.a.kawi-move",
    ))]);
    let decoded = ApplyJournal::from_bytes(&journal.to_bytes().unwrap()).unwrap();
    assert_eq!(decoded, journal);
    assert_eq!(
        decoded.cleanups(),
        vec![JournalCleanup::remove(PathBuf::from(
            "/project/.a.kawi-move"
        ))]
    );
}

#[test]
fn legacy_deferred_paths_are_read_as_cleanups() {
    let bytes =
        br#"{"version":1,"phase":"committed","operations":[],"deferred":["/project/.a.kawi-move"]}"#;
    let journal = ApplyJournal::from_bytes(bytes).unwrap();
    assert_eq!(
        journal.cleanups(),
        vec![JournalCleanup::remove(PathBuf::from(
            "/project/.a.kawi-move"
        ))]
    );
}

#[test]
fn corrupt_and_relative_journals_are_rejected() {
    assert!(ApplyJournal::from_bytes(b"not-json").is_err());
    let relative = ApplyJournal::prepared(vec![JournalOperation::CreateFile {
        path: JournalPath::from(PathBuf::from("relative")),
    }]);
    assert!(ApplyJournal::from_bytes(&relative.to_bytes().unwrap()).is_err());
    let mut unsupported = journal();
    unsupported.version = APPLY_JOURNAL_VERSION + 1;
    assert!(ApplyJournal::from_bytes(&unsupported.to_bytes().unwrap()).is_err());
    let mut cleanup = journal();
    cleanup.cleanup = vec![JournalCleanup::remove(PathBuf::from("relative"))];
    assert!(ApplyJournal::from_bytes(&cleanup.to_bytes().unwrap()).is_err());
}

#[cfg(unix)]
#[test]
fn non_utf8_paths_round_trip() {
    use std::os::unix::ffi::OsStringExt;
    let path = PathBuf::from(std::ffi::OsString::from_vec(b"/tmp/bad-\xff".to_vec()));
    let journal = ApplyJournal::prepared(vec![JournalOperation::CreateFile {
        path: JournalPath::from(path.clone()),
    }]);
    let decoded = ApplyJournal::from_bytes(&journal.to_bytes().unwrap()).unwrap();
    let JournalOperation::CreateFile { path: decoded_path } = &decoded.operations[0] else {
        panic!("unexpected operation");
    };
    assert_eq!(decoded_path.as_path(), path);
}
