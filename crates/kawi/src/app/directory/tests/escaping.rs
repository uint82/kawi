use std::fs;

use super::{listing, set_listing, settle, setup};
use crate::app::save::SaveAfter;

#[test]
fn newline_and_tab_names_round_trip_through_the_buffer() {
    let original_name = "line\nbreak\ttext";
    let (dir, mut app) = setup("directory_escape_round_trip", &[original_name], &[]);
    assert_eq!(listing(&app), [r"line\nbreak\ttext"]);
    set_listing(&mut app, r"renamed\nname");
    assert!(!app.request_save(false, SaveAfter::None, true));
    settle(&mut app);
    assert!(dir.join("renamed\nname").exists());
    assert!(!dir.join(original_name).exists());
    assert_eq!(listing(&app), [r"renamed\nname"]);
    let _ = fs::remove_dir_all(dir);
}

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn invalid_utf8_names_are_escaped_and_renamed_losslessly() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    use kawi_buffer::Buffer;

    use crate::app::App;
    use crate::keymap::Flavor;
    use crate::test_support::temp_dir;

    let dir = temp_dir("directory_escape_invalid_utf8");
    let original = dir.join(OsString::from_vec(b"raw\xffname".to_vec()));
    fs::write(&original, "content\n").unwrap();
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    assert!(app.open_directory_path(&dir, true));
    settle(&mut app);
    assert_eq!(listing(&app), [r"raw\xffname"]);
    set_listing(&mut app, "renamed.txt");
    assert!(!app.request_save(false, SaveAfter::None, true));
    settle(&mut app);
    assert!(dir.join("renamed.txt").exists());
    assert!(!original.exists());
    let _ = fs::remove_dir_all(dir);
}
