use std::fs;
use std::slice::from_ref;

use super::{listing, set_listing, settle, setup};

#[test]
fn forced_edit_discards_pending_directory_changes() {
    let (dir, mut app) = setup("directory_force_reload", &["alpha.txt"], &[]);
    let number = app.editor.buffer_number();
    set_listing(&mut app, "beta.txt");
    assert!(app.handle_directory_watch_paths(from_ref(&dir.join("ghost.txt"))));
    assert_eq!(app.disk_status(number), Some("changed"));
    assert!(!app.handle_edit(None, true).unwrap());
    settle(&mut app);
    assert_eq!(listing(&app), ["alpha.txt"]);
    assert!(!app.editor.is_dirty());
    assert_eq!(app.disk_status(number), None);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn edit_without_force_keeps_pending_directory_changes() {
    let (dir, mut app) = setup("directory_plain_reload", &["alpha.txt"], &[]);
    set_listing(&mut app, "beta.txt");
    assert!(!app.handle_edit(None, false).unwrap());
    settle(&mut app);
    assert_eq!(listing(&app), ["beta.txt"]);
    assert!(app.editor.is_dirty());
    assert_eq!(
        app.last_message(),
        Some("Save or undo changes before reloading the directory")
    );
    fs::remove_dir_all(dir).unwrap();
}
