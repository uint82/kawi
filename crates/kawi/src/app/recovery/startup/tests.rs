use std::fs;

use kawi_buffer::Buffer;

use super::*;
use crate::keymap::Flavor;
use crate::test_support::temp_dir;

fn app_with_text(text: &str) -> App {
    App::new(Buffer::from_string(text), Flavor::Vim)
}

#[test]
fn scheduled_recovery_starts_only_when_requested() {
    let root = temp_dir("startup_recovery");
    let cwd = root.join("workspace");
    fs::create_dir_all(&cwd).unwrap();
    let mut app = app_with_text("alpha\n");
    app.schedule_startup_recovery(root.clone(), cwd, false);
    assert!(app.recovery.is_none());
    assert!(app.pending_startup_recovery.is_some());
    assert!(app.start_pending_recovery());
    assert!(app.recovery.is_some());
    assert!(app.pending_startup_recovery.is_none());
    assert!(!app.start_pending_recovery());
    drop(app);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn scheduled_recovery_warning_reaches_the_message_line() {
    let root = temp_dir("startup_recovery_warning");
    let cwd = root.join("workspace");
    fs::create_dir_all(&cwd).unwrap();
    let blocked = root.join("blocked");
    fs::write(&blocked, b"not a directory").unwrap();
    let mut app = app_with_text("alpha\n");
    app.schedule_startup_recovery(blocked, cwd, false);
    assert!(app.last_message().is_none());
    assert!(app.start_pending_recovery());
    assert!(app
        .last_message()
        .is_some_and(|message| message.contains("recovery")));
    drop(app);
    fs::remove_dir_all(root).unwrap();
}
