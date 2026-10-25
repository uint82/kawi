use std::fs;
use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;
use kawi_editor::{parse, Action};

mod delete;
mod rename;

use super::App;
use crate::keymap::Flavor;

fn temp_dir(name: &str) -> PathBuf {
    crate::test_support::temp_dir(name)
}

fn app_for(path: &Path) -> App {
    App::new(
        Buffer::from_file(path.to_str().expect("path")).expect("buffer"),
        Flavor::Vim,
    )
}

fn edit(app: &mut App, text: &str) {
    app.editor.apply(Action::EnterInsert);
    for character in text.chars() {
        app.editor.apply(Action::InsertChar(character));
    }
    app.editor.apply(Action::ExitInsert);
}

fn command(app: &mut App, text: &str) {
    app.run_command(parse(text)).expect("command");
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn message_text(app: &App) -> String {
    app.last_message()
        .map(str::to_string)
        .or_else(|| app.pager_lines().map(|lines| lines.concat()))
        .unwrap_or_default()
}

fn type_text(app: &mut App, text: &str) {
    for character in text.chars() {
        app.handle_key(key(KeyCode::Char(character))).expect("key");
    }
}

#[test]
fn save_as_writes_a_copy_and_adopts_the_new_path() {
    let dir = temp_dir("save_as");
    let source = dir.join("a.txt");
    let target = dir.join("copy.txt");
    fs::write(&source, "alpha\n").unwrap();
    let mut app = app_for(&source);
    edit(&mut app, "x");
    command(&mut app, &format!("saveas {}", target.display()));
    assert_eq!(fs::read_to_string(&source).unwrap(), "alpha\n");
    assert_eq!(fs::read_to_string(&target).unwrap(), "xalpha\n");
    assert_eq!(app.editor.filename(), target.to_string_lossy());
    assert!(!app.editor.is_dirty());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn save_as_refuses_an_existing_target_without_force() {
    let dir = temp_dir("save_as_exists");
    let source = dir.join("a.txt");
    let target = dir.join("exists.txt");
    fs::write(&source, "alpha\n").unwrap();
    fs::write(&target, "old\n").unwrap();
    let mut app = app_for(&source);
    command(&mut app, &format!("saveas {}", target.display()));
    assert_eq!(
        app.last_message(),
        Some("E13: File exists (add ! to override)")
    );
    assert_eq!(app.editor.filename(), source.to_string_lossy());
    command(&mut app, &format!("saveas! {}", target.display()));
    assert_eq!(fs::read_to_string(&target).unwrap(), "alpha\n");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn save_as_prompts_before_creating_missing_parents() {
    let dir = temp_dir("save_as_parents");
    let source = dir.join("a.txt");
    let target = dir.join("nested/copy.txt");
    fs::write(&source, "alpha\n").unwrap();
    let mut app = app_for(&source);
    command(&mut app, &format!("saveas {}", target.display()));
    assert!(app.file_ops.is_some());
    assert!(!dir.join("nested").exists());
    app.handle_key(key(KeyCode::Char('y'))).expect("confirm");
    assert_eq!(fs::read_to_string(&target).unwrap(), "alpha\n");
    assert_eq!(app.editor.filename(), target.to_string_lossy());
    assert_eq!(fs::read_to_string(&source).unwrap(), "alpha\n");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn write_prompts_before_creating_missing_parents() {
    let dir = temp_dir("write_parents");
    let target = dir.join("nested/new.txt");
    let mut app = App::new(Buffer::from_string("alpha"), Flavor::Vim);
    command(&mut app, &format!("w {}", target.display()));
    assert!(app.file_ops.is_some());
    assert!(!target.exists());
    app.handle_key(key(KeyCode::Char('y'))).expect("confirm");
    assert_eq!(fs::read_to_string(&target).unwrap(), "alpha\n");
    assert!(app.file_ops.is_none());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn declining_parent_creation_writes_nothing() {
    let dir = temp_dir("write_parents_no");
    let target = dir.join("nested/new.txt");
    let mut app = App::new(Buffer::from_string("alpha"), Flavor::Vim);
    command(&mut app, &format!("w {}", target.display()));
    app.handle_key(key(KeyCode::Char('n'))).expect("decline");
    assert!(!target.exists());
    assert!(app.file_ops.is_none());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn forced_write_creates_parents_without_a_prompt() {
    let dir = temp_dir("write_parents_force");
    let target = dir.join("nested/new.txt");
    let mut app = App::new(Buffer::from_string("alpha"), Flavor::Vim);
    command(&mut app, &format!("w! {}", target.display()));
    assert_eq!(fs::read_to_string(&target).unwrap(), "alpha\n");
    assert!(app.file_ops.is_none());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn declining_parent_creation_creates_nothing() {
    let dir = temp_dir("create_decline");
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    app.create_new_path(&dir, "nested/new.txt");
    app.handle_key(key(KeyCode::Char('n'))).expect("decline");
    assert!(!dir.join("nested").exists());
    assert!(app.file_ops.is_none());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn picker_new_file_opens_a_prompt_in_the_selected_directory() {
    let dir = temp_dir("picker_new");
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/main.rs"), "fn main() {}\n").unwrap();
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    app.picker = Some(crate::app::picker::PickerSession::test_files(
        dir.clone(),
        vec!["src/main.rs".to_string()],
    ));
    app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL))
        .expect("new");
    let prompt = app.file_ops_prompt().expect("prompt");
    assert!(prompt.value.ends_with("src/"));
    assert!(app.picker.is_none());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn picker_rename_prefills_the_selected_file() {
    let dir = temp_dir("picker_rename");
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/main.rs"), "fn main() {}\n").unwrap();
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    app.picker = Some(crate::app::picker::PickerSession::test_files(
        dir.clone(),
        vec!["src/main.rs".to_string()],
    ));
    app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL))
        .expect("rename");
    let prompt = app.file_ops_prompt().expect("prompt");
    assert!(prompt.value.ends_with("src/main.rs"));
    assert!(app.picker.is_none());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn picker_delete_opens_a_confirmation_for_the_selected_file() {
    let dir = temp_dir("picker_delete");
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/main.rs"), "fn main() {}\n").unwrap();
    let mut app = app_for(&dir.join("src/main.rs"));
    app.picker = Some(crate::app::picker::PickerSession::test_files(
        dir.clone(),
        vec!["src/main.rs".to_string()],
    ));
    app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL))
        .expect("delete");
    assert!(app.file_ops_prompt().is_some());
    assert!(app.picker.is_none());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn create_new_path_creates_files_and_directories() {
    let dir = temp_dir("create");
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    app.create_new_path(&dir, "nested/new.txt");
    assert!(app.file_ops_prompt().is_some());
    assert!(!dir.join("nested").exists());
    app.handle_key(key(KeyCode::Char('y'))).expect("confirm");
    assert_eq!(fs::read_to_string(dir.join("nested/new.txt")).unwrap(), "");
    assert_eq!(
        app.editor.filename(),
        dir.join("nested/new.txt").to_string_lossy()
    );
    app.create_new_path(&dir, "another/");
    assert!(dir.join("another").is_dir());
    app.create_new_path(&dir, "another/");
    assert_eq!(
        app.last_message(),
        Some("E13: File exists (add ! to override)")
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn creating_a_file_does_not_report_its_own_watcher_event() {
    let dir = temp_dir("create_watch");
    let target = dir.join("new.txt");
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    app.create_new_path(&dir, "new.txt");
    assert_eq!(
        app.last_message(),
        Some(format!("Created \"{}\"", target.display())).as_deref()
    );
    app.handle_disk_paths(std::slice::from_ref(&target));
    assert_eq!(
        app.last_message(),
        Some(format!("Created \"{}\"", target.display())).as_deref()
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn normal_mode_ctrl_r_still_redoes() {
    let dir = temp_dir("ctrl_r_redo");
    let path = dir.join("a.txt");
    fs::write(&path, "alpha\n").unwrap();
    let mut app = app_for(&path);
    edit(&mut app, "x");
    app.editor.apply(Action::Undo);
    assert_eq!(app.editor.buffer_text(1).unwrap(), "alpha\n");
    app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL))
        .expect("redo");
    assert_eq!(app.editor.buffer_text(1).unwrap(), "xalpha\n");
    assert!(app.file_ops_prompt().is_none());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn insert_mode_ctrl_d_still_shifts_left() {
    let dir = temp_dir("ctrl_d_shift");
    let path = dir.join("a.txt");
    fs::write(&path, "\tab\n").unwrap();
    let mut app = app_for(&path);
    app.editor.apply(Action::EnterInsert);
    app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL))
        .expect("shift");
    assert_eq!(app.editor.buffer_text(1).unwrap(), "ab\n");
    assert!(app.file_ops_prompt().is_none());
    let _ = fs::remove_dir_all(dir);
}
