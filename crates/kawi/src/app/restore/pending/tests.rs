use std::fs;

use super::*;
use crate::test_support::temp_dir;

#[test]
fn restore_warnings_cover_singular_plural_and_interruption() {
    assert_eq!(
        restore_warning(1, false).as_deref(),
        Some("session: skipped 1 unavailable buffer")
    );
    assert_eq!(
        restore_warning(2, false).as_deref(),
        Some("session: skipped 2 unavailable buffers")
    );
    assert_eq!(
        restore_warning(0, true).as_deref(),
        Some("session: buffer loader stopped early")
    );
    assert_eq!(restore_warning(0, false), None);
}

#[test]
fn load_buffer_reads_the_pending_path() {
    let dir = temp_dir("session_pending_load");
    let path = dir.join("alpha.txt");
    fs::write(&path, "alpha\n").unwrap();
    let loaded = load_buffer(PendingBufferLoad {
        number: 7,
        path: path.to_string_lossy().into_owned(),
    })
    .unwrap();
    assert_eq!(loaded.number, 7);
    assert_eq!(loaded.buffer.snapshot().to_string(), "alpha\n");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn load_buffer_reports_io_failures() {
    let dir = temp_dir("session_pending_failure");
    let path = dir.join("directory");
    fs::create_dir(&path).unwrap();
    assert!(load_buffer(PendingBufferLoad {
        number: 7,
        path: path.to_string_lossy().into_owned(),
    })
    .is_err());
    fs::remove_dir_all(dir).unwrap();
}
