use std::thread;
use std::time::{Duration, Instant};

use super::*;

fn finish_restore(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.session_restore.is_some() {
        app.poll_session_restore();
        if app.session_restore.is_some() {
            thread::sleep(Duration::from_millis(1));
        }
        assert!(Instant::now() < deadline, "session restore timed out");
    }
}

#[test]
fn hidden_buffers_load_after_restore_and_keep_session_order() {
    let dir = temp_dir("lazy_hidden");
    let alpha = write_file(&dir, "alpha.txt", "alpha\n");
    let bravo = write_file(&dir, "bravo.txt", "bravo\n");
    let charlie = write_file(&dir, "charlie.txt", "charlie\n");
    let session = session(
        vec![
            SessionBuffer {
                path: Some(alpha.clone()),
                id: Some("alpha-id".to_string()),
            },
            SessionBuffer {
                path: Some(bravo.clone()),
                id: Some("bravo-id".to_string()),
            },
            SessionBuffer {
                path: Some(charlie.clone()),
                id: Some("charlie-id".to_string()),
            },
        ],
        leaf(1, 0, 0, 0),
        1,
    );
    let mut app = empty_app();
    assert_eq!(app.restore_session(&session), None);
    assert_eq!(app.editor.buffer_numbers(), vec![1, 2, 3]);
    assert_eq!(app.editor.buffer_text(1).as_deref(), Some("alpha\n"));
    assert_eq!(app.editor.buffer_text(2).as_deref(), Some(""));
    assert_eq!(app.editor.buffer_text(3).as_deref(), Some(""));
    assert_eq!(app.pending_session_buffers.len(), 2);
    assert!(app.start_pending_session_restore());
    finish_restore(&mut app);
    assert_eq!(app.editor.buffer_text(2).as_deref(), Some("bravo\n"));
    assert_eq!(app.editor.buffer_text(3).as_deref(), Some("charlie\n"));
    assert_eq!(app.editor.buffer_id(2), Some("bravo-id"));
    assert_eq!(app.editor.buffer_id(3), Some("charlie-id"));
    assert!(!app.editor.is_buffer_pending(2));
    assert!(!app.editor.is_buffer_pending(3));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn visible_split_buffers_load_before_restore_returns() {
    let dir = temp_dir("lazy_visible");
    let alpha = write_file(&dir, "alpha.txt", "alpha\n");
    let bravo = write_file(&dir, "bravo.txt", "bravo\n");
    let charlie = write_file(&dir, "charlie.txt", "charlie\n");
    let session = session(
        vec![
            SessionBuffer {
                path: Some(alpha.clone()),
                id: None,
            },
            SessionBuffer {
                path: Some(bravo.clone()),
                id: None,
            },
            SessionBuffer {
                path: Some(charlie),
                id: None,
            },
        ],
        split(leaf(1, 0, 0, 0), leaf(2, 1, 0, 0)),
        2,
    );
    let mut app = empty_app();
    assert_eq!(app.restore_session(&session), None);
    assert_eq!(app.windows.len(), 2);
    assert_eq!(app.editor.buffer_text(1).as_deref(), Some("alpha\n"));
    assert_eq!(app.editor.buffer_text(2).as_deref(), Some("bravo\n"));
    assert_eq!(app.editor.buffer_text(3).as_deref(), Some(""));
    assert_eq!(app.pending_session_buffers.len(), 1);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn selecting_a_pending_buffer_loads_it_on_demand() {
    let dir = temp_dir("lazy_select");
    let alpha = write_file(&dir, "alpha.txt", "alpha\n");
    let bravo = write_file(&dir, "bravo.txt", "bravo\n");
    let session = session(
        vec![
            SessionBuffer {
                path: Some(alpha),
                id: None,
            },
            SessionBuffer {
                path: Some(bravo),
                id: None,
            },
        ],
        leaf(1, 0, 0, 0),
        1,
    );
    let mut app = empty_app();
    assert_eq!(app.restore_session(&session), None);
    assert!(app.editor.is_buffer_pending(2));
    assert!(app.editor.buffer_select(2));
    assert_eq!(app.editor.buffer_text(2).as_deref(), Some("bravo\n"));
    assert!(!app.editor.is_buffer_pending(2));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn missing_hidden_buffers_are_skipped_before_scheduling() {
    let dir = temp_dir("lazy_missing");
    let alpha = write_file(&dir, "alpha.txt", "alpha\n");
    let missing = dir.join("missing.txt").to_string_lossy().into_owned();
    let session = session(
        vec![
            SessionBuffer {
                path: Some(alpha),
                id: None,
            },
            SessionBuffer {
                path: Some(missing),
                id: None,
            },
        ],
        leaf(1, 0, 0, 0),
        1,
    );
    let mut app = empty_app();
    assert_eq!(
        app.restore_session(&session),
        Some("session: skipped 1 unavailable buffer".to_string())
    );
    assert_eq!(app.editor.buffer_numbers(), vec![1]);
    assert!(app.pending_session_buffers.is_empty());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn background_load_failure_reports_a_warning() {
    let dir = temp_dir("lazy_failure");
    let alpha = write_file(&dir, "alpha.txt", "alpha\n");
    let hidden = dir.join("hidden.txt");
    fs::write(&hidden, "hidden\n").unwrap();
    let hidden_text = hidden.to_string_lossy().into_owned();
    let session = session(
        vec![
            SessionBuffer {
                path: Some(alpha),
                id: None,
            },
            SessionBuffer {
                path: Some(hidden_text),
                id: None,
            },
        ],
        leaf(1, 0, 0, 0),
        1,
    );
    let mut app = empty_app();
    assert_eq!(app.restore_session(&session), None);
    fs::remove_file(&hidden).unwrap();
    fs::create_dir(&hidden).unwrap();
    assert!(app.start_pending_session_restore());
    finish_restore(&mut app);
    assert!(app
        .last_message()
        .is_some_and(|message| message.contains("skipped 1 unavailable buffer")));
    fs::remove_dir_all(dir).unwrap();
}
