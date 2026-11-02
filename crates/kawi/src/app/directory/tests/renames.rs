use std::fs;
use std::slice::from_ref;

use super::super::test_support::message_text;
use super::{listing, settle, setup};

fn moved_path(dir: &std::path::Path, suffix: &str) -> std::path::PathBuf {
    dir.with_file_name(format!(
        "{}_{suffix}",
        dir.file_name().unwrap().to_string_lossy()
    ))
}

#[test]
fn root_renames_retarget_directory_buffers() {
    let (dir, mut app) = setup("directory_rename_root", &["alpha.txt"], &[]);
    let number = app.editor.buffer_number();
    assert!(app.directory.sync_watch().is_none());
    let moved = moved_path(&dir, "moved");
    fs::rename(&dir, &moved).unwrap();
    assert!(app.handle_directory_watch_paths(from_ref(&dir)));
    assert_eq!(app.directory.get(number).unwrap().root, moved);
    assert_eq!(app.editor.filename(), moved.to_string_lossy());
    assert_eq!(listing(&app), ["alpha.txt"]);
    assert_eq!(app.disk_status(number), None);
    assert!(message_text(&app).starts_with("Directory moved on disk"));
    fs::remove_dir_all(moved).unwrap();
}

#[test]
fn nested_directory_buffers_follow_root_renames() {
    let (dir, mut app) = setup("directory_rename_nested", &["alpha.txt"], &["sub"]);
    fs::write(dir.join("sub/nested.txt"), "nested\n").unwrap();
    let root_number = app.editor.buffer_number();
    assert!(app.open_directory_path(&dir.join("sub"), true));
    settle(&mut app);
    let nested_number = app.editor.buffer_number();
    assert!(app.directory.sync_watch().is_none());
    let moved = moved_path(&dir, "moved");
    fs::rename(&dir, &moved).unwrap();
    assert!(app.handle_directory_watch_paths(from_ref(&dir)));
    settle(&mut app);
    assert_eq!(app.directory.get(root_number).unwrap().root, moved);
    assert_eq!(
        app.directory.get(nested_number).unwrap().root,
        moved.join("sub")
    );
    assert_eq!(app.editor.filename(), moved.join("sub").to_string_lossy());
    assert_eq!(listing(&app), ["nested.txt"]);
    assert_eq!(app.disk_status(root_number), None);
    assert_eq!(app.disk_status(nested_number), None);
    fs::remove_dir_all(moved).unwrap();
}
