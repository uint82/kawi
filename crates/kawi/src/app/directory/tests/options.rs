use std::fs;

use kawi_editor::{Action, SetValue};

use super::{listing, set_listing, settle, setup, SaveAfter};
use crate::app::App;

fn assign(app: &mut App, name: &str, value: &str) {
    app.handle_set(name, SetValue::Assign(Some(value.to_string())))
        .unwrap();
}

fn enable(app: &mut App, name: &str) {
    app.handle_set(name, SetValue::Assign(None)).unwrap();
}

#[test]
fn sort_changes_reload_clean_buffers_and_keep_selection() {
    let (dir, mut app) = setup(
        "directory_live_sort",
        &["alpha.txt", "beta.txt", "gamma.txt"],
        &[],
    );
    fs::write(dir.join("alpha.txt"), "a").unwrap();
    fs::write(dir.join("beta.txt"), "bbbbbb").unwrap();
    fs::write(dir.join("gamma.txt"), "ccc").unwrap();
    app.editor.apply(Action::JumpTo(1, 0));
    assign(&mut app, "directory_sort", "size");
    settle(&mut app);
    assert_eq!(listing(&app), ["alpha.txt", "gamma.txt", "beta.txt"]);
    assert_eq!(listing(&app)[app.editor.row()], "beta.txt");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn hidden_entries_reload_when_visibility_changes() {
    let (dir, mut app) = setup("directory_live_hidden", &[".hidden", "alpha.txt"], &[]);
    assert_eq!(listing(&app), ["alpha.txt"]);
    enable(&mut app, "directory_show_hidden");
    settle(&mut app);
    assert_eq!(listing(&app), [".hidden", "alpha.txt"]);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn metadata_changes_reload_clean_buffers() {
    let (dir, mut app) = setup("directory_live_metadata", &["alpha.txt"], &[]);
    let number = app.editor.buffer_number();
    let columns = app.directory_options.metadata;
    assert!(app.directory.get(number).unwrap().metadata_width(columns) > 0);
    assign(&mut app, "directory_metadata", "none");
    settle(&mut app);
    let columns = app.directory_options.metadata;
    assert_eq!(
        app.directory.get(number).unwrap().metadata_width(columns),
        0
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn dirty_buffers_keep_their_listing_and_report_the_change() {
    let (dir, mut app) = setup("directory_live_dirty", &["alpha.txt"], &[]);
    let number = app.editor.buffer_number();
    set_listing(&mut app, "renamed.txt");
    let revision = app.editor.buffer_revision_at(number).unwrap();
    enable(&mut app, "directory_show_hidden");
    settle(&mut app);
    assert_eq!(listing(&app), ["renamed.txt"]);
    assert_eq!(
        app.last_message(),
        Some("Directory option changed; save or undo changes before reloading")
    );
    assert_eq!(app.editor.buffer_revision_at(number), Some(revision));
    assert!(!app.directory.get(number).unwrap().show_hidden);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn saving_a_dirty_buffer_applies_the_new_options() {
    let (dir, mut app) = setup(
        "directory_live_dirty_save",
        &["alpha.txt", "beta.txt", "gamma.txt"],
        &[],
    );
    set_listing(&mut app, "delta.txt\nbeta.txt\ngamma.txt");
    assign(&mut app, "directory_sort_reverse", "true");
    assert!(!app.request_save(false, SaveAfter::None, true));
    settle(&mut app);
    assert_eq!(listing(&app), ["gamma.txt", "delta.txt", "beta.txt"]);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn non_listing_and_unchanged_options_do_not_reload() {
    let (dir, mut app) = setup("directory_live_noop", &["alpha.txt"], &[]);
    let number = app.editor.buffer_number();
    let revision = app.editor.buffer_revision_at(number).unwrap();
    assign(&mut app, "directory_confirm", "false");
    assign(&mut app, "directory_sort", "name");
    settle(&mut app);
    assert_eq!(app.editor.buffer_revision_at(number), Some(revision));
    assert!(!app.directory_options.confirm);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn per_buffer_hidden_toggle_survives_an_explicit_reload() {
    let (dir, mut app) = setup("directory_live_toggle", &[".hidden", "alpha.txt"], &[]);
    assert!(app.toggle_directory_hidden());
    settle(&mut app);
    assert!(app
        .start_directory_reload(app.editor.buffer_number())
        .is_ok());
    settle(&mut app);
    assert_eq!(listing(&app), [".hidden", "alpha.txt"]);
    fs::remove_dir_all(dir).unwrap();
}
