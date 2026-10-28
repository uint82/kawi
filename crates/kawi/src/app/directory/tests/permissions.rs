#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;

use super::super::test_support::message_text;
use super::{set_listing, settle, setup, SaveAfter};
use crate::test_support::temp_dir;

#[test]
fn plan_preflight_blocks_a_read_only_directory() {
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let (dir, mut app) = setup("directory_permission_plan", &["alpha.txt"], &[]);
    set_listing(&mut app, "beta.txt");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();
    assert!(!app.request_save(false, SaveAfter::None, true));
    settle(&mut app);
    let message = message_text(&app);
    assert!(message.contains("requires write and execute permission"));
    assert!(message.contains(&dir.to_string_lossy().into_owned()));
    assert!(dir.join("alpha.txt").exists());
    assert!(!dir.join("beta.txt").exists());
    assert!(app.editor.has_unsaved());
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn apply_preflight_catches_permissions_changed_after_confirmation() {
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let (dir, mut app) = setup("directory_permission_apply", &["alpha.txt"], &[]);
    set_listing(&mut app, "beta.txt\nnew.txt");
    assert!(!app.request_save(false, SaveAfter::None, true));
    settle(&mut app);
    assert!(app.directory.has_pending());
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();
    app.apply_pending_directory(true);
    settle(&mut app);
    let message = message_text(&app);
    assert!(message.contains("requires write and execute permission"));
    assert!(message.contains("no operations were applied"));
    assert!(dir.join("alpha.txt").exists());
    assert!(!dir.join("beta.txt").exists());
    assert!(!dir.join("new.txt").exists());
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn save_all_stops_before_writes_when_one_directory_is_read_only() {
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let (first, mut app) = setup(
        "directory_permission_wall_first",
        &["alpha.txt", "notes.txt"],
        &[],
    );
    set_listing(&mut app, "beta.txt\nnotes.txt");
    let second = temp_dir("directory_permission_wall_second");
    fs::write(second.join("gamma.txt"), "content\n").unwrap();
    assert!(app.open_directory_path(&second, true));
    settle(&mut app);
    set_listing(&mut app, "delta.txt");
    let notes = first.join("notes.txt");
    app.editor
        .open_file_buffer(notes.to_string_lossy().as_ref())
        .unwrap();
    app.editor.apply(kawi_editor::Action::EnterInsert);
    app.editor.apply(kawi_editor::Action::InsertChar('x'));
    app.editor.apply(kawi_editor::Action::ExitInsert);
    fs::set_permissions(&second, fs::Permissions::from_mode(0o555)).unwrap();
    assert!(!app.handle_save_all().unwrap());
    settle(&mut app);
    let message = message_text(&app);
    assert!(message.contains("requires write and execute permission"));
    assert!(message.contains(&second.to_string_lossy().into_owned()));
    assert!(first.join("alpha.txt").exists());
    assert!(!first.join("beta.txt").exists());
    assert!(second.join("gamma.txt").exists());
    assert!(!second.join("delta.txt").exists());
    assert_eq!(fs::read_to_string(&notes).unwrap(), "content\n");
    assert!(app.editor.has_unsaved());
    fs::set_permissions(&second, fs::Permissions::from_mode(0o755)).unwrap();
    let _ = fs::remove_dir_all(first);
    let _ = fs::remove_dir_all(second);
}

#[cfg(unix)]
#[test]
fn plan_preflight_rejects_a_parent_without_search_permission() {
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let (dir, mut app) = setup("directory_permission_search", &["alpha.txt"], &[]);
    set_listing(&mut app, "beta.txt");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(!app.request_save(false, SaveAfter::None, true));
    settle(&mut app);
    let message = message_text(&app);
    assert!(
        message.contains("write and execute permission"),
        "{message}"
    );
    assert!(
        message.contains(&dir.to_string_lossy().into_owned()),
        "{message}"
    );
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(dir.join("alpha.txt").exists());
    assert!(!dir.join("beta.txt").exists());
    let _ = fs::remove_dir_all(dir);
}
