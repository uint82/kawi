use std::fs;

use crossterm::event::KeyCode;
use kawi_buffer::Buffer;

use super::super::test_support::{message_text, settle};
use super::{key, listing};
use crate::app::App;
use crate::keymap::Flavor;
use crate::test_support::temp_dir;

#[cfg(unix)]
#[test]
fn enter_refuses_to_open_special_files() {
    use std::os::unix::net::UnixListener;

    let dir = temp_dir("directory_special_open");
    let _listener = UnixListener::bind(dir.join("sock")).unwrap();
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    assert!(app.open_directory_path(&dir, true));
    settle(&mut app);
    assert_eq!(listing(&app), ["sock"]);
    assert!(app.handle_directory_key(key(KeyCode::Enter)).unwrap());
    assert!(message_text(&app).contains("Cannot open special file: sock"));
    assert_eq!(app.editor.filename(), dir.to_string_lossy().as_ref());
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn enter_reports_broken_symlinks() {
    use std::os::unix::fs::symlink;

    let dir = temp_dir("directory_broken_symlink_open");
    symlink("missing", dir.join("link")).unwrap();
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    assert!(app.open_directory_path(&dir, true));
    settle(&mut app);
    assert_eq!(listing(&app), ["link"]);
    assert!(app.handle_directory_key(key(KeyCode::Enter)).unwrap());
    assert!(message_text(&app).contains("Broken symlink: link"));
    assert_eq!(app.editor.filename(), dir.to_string_lossy().as_ref());
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn enter_opens_symlinks_to_regular_files() {
    use std::os::unix::fs::symlink;

    let dir = temp_dir("directory_symlink_open");
    fs::write(dir.join("target.txt"), "content\n").unwrap();
    symlink("target.txt", dir.join("link")).unwrap();
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    assert!(app.open_directory_path(&dir, true));
    settle(&mut app);
    assert_eq!(listing(&app), ["link", "target.txt"]);
    assert!(app.handle_directory_key(key(KeyCode::Enter)).unwrap());
    settle(&mut app);
    assert_eq!(
        app.editor.filename(),
        dir.join("link").to_string_lossy().as_ref()
    );
    fs::remove_dir_all(dir).unwrap();
}
