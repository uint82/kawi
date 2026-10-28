use std::fs;

use super::super::save::SaveAfter;
use super::test_support::settle;
use super::tests::{listing, set_listing, setup};

#[test]
fn background_save_checks_reject_external_changes() {
    let (dir, mut app) = setup("directory_external", &["alpha.txt"], &[]);
    set_listing(&mut app, "beta.txt");
    fs::write(dir.join("external.txt"), "external\n").unwrap();
    assert!(!app.request_save(false, SaveAfter::None, true));
    settle(&mut app);
    assert!(dir.join("alpha.txt").exists());
    assert!(!dir.join("beta.txt").exists());
    assert_eq!(
        app.last_message(),
        Some("Directory changed on disk; reload before saving")
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn background_reloads_show_external_changes() {
    let (dir, mut app) = setup("directory_reload", &["alpha.txt"], &[]);
    fs::write(dir.join("beta.txt"), "new\n").unwrap();
    assert!(app
        .start_directory_reload(app.editor.buffer_number())
        .is_ok());
    settle(&mut app);
    assert_eq!(listing(&app), ["alpha.txt", "beta.txt"]);
    let _ = fs::remove_dir_all(dir);
}
