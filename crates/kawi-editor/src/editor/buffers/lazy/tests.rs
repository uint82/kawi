use std::fs;
use std::path::{Path, PathBuf};

use kawi_buffer::Buffer;

use crate::Editor;

fn temp_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("kawi_lazy_buffer_{}_{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn file_path(dir: &Path, name: &str) -> String {
    dir.join(name).to_string_lossy().into_owned()
}

fn editor() -> Editor {
    Editor::new(Buffer::from_string("initial\n"))
}

#[test]
fn placeholder_defers_the_file_read_until_selection() {
    let dir = temp_dir("defer");
    fs::write(dir.join("alpha.txt"), "disk\n").unwrap();
    let path = file_path(&dir, "alpha.txt");
    let mut editor = editor();
    let number = editor.add_file_placeholder(&path);
    assert_eq!(editor.buffer_number(), 1);
    assert_eq!(editor.buffer_text(number).as_deref(), Some(""));
    assert!(editor.is_buffer_pending(number));
    assert!(editor.buffer_select(number));
    assert_eq!(editor.buffer_text(number).as_deref(), Some("disk\n"));
    assert!(!editor.is_buffer_pending(number));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn replacing_a_parked_buffer_preserves_identity_and_clears_pending() {
    let dir = temp_dir("replace");
    fs::write(dir.join("alpha.txt"), "disk\n").unwrap();
    let path = file_path(&dir, "alpha.txt");
    let mut editor = editor();
    let number = editor.add_file_placeholder(&path);
    assert!(editor.set_buffer_id(number, "session-id".to_string()));
    assert!(editor.replace_parked_buffer(number, Buffer::from_string("loaded\n")));
    assert_eq!(editor.buffer_id(number), Some("session-id"));
    assert_eq!(editor.buffer_text(number).as_deref(), Some("loaded\n"));
    assert!(!editor.is_buffer_pending(number));
    assert!(!editor.replace_parked_buffer(number, Buffer::from_string("again\n")));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn placeholder_paths_are_deduplicated() {
    let dir = temp_dir("deduplicate");
    fs::write(dir.join("alpha.txt"), "alpha\n").unwrap();
    let path = file_path(&dir, "alpha.txt");
    let mut editor = editor();
    let first = editor.add_file_placeholder(&path);
    let second = editor.add_file_placeholder(&path);
    assert_eq!(first, second);
    assert_eq!(editor.buffer_numbers(), vec![1, first]);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn selecting_a_missing_placeholder_finishes_loading_without_content() {
    let dir = temp_dir("missing");
    let path = file_path(&dir, "missing.txt");
    let mut editor = editor();
    let number = editor.add_file_placeholder(&path);
    assert!(editor.buffer_select(number));
    assert_eq!(editor.buffer_text(number).as_deref(), Some(""));
    assert!(!editor.is_buffer_pending(number));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn replacing_an_unknown_buffer_is_rejected() {
    let mut editor = editor();
    assert!(!editor.replace_parked_buffer(99, Buffer::from_string("loaded\n")));
    assert!(!editor.is_buffer_pending(99));
}

#[test]
fn reloading_a_pending_buffer_loads_content_and_clears_pending() {
    let dir = temp_dir("reload");
    fs::write(dir.join("alpha.txt"), "disk\n").unwrap();
    let path = file_path(&dir, "alpha.txt");
    let mut editor = editor();
    let number = editor.add_file_placeholder(&path);
    assert!(editor.reload_buffer(number).is_ok());
    assert_eq!(editor.buffer_text(number).as_deref(), Some("disk\n"));
    assert!(!editor.is_buffer_pending(number));
    fs::remove_dir_all(dir).unwrap();
}
