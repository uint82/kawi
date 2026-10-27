use std::fs;
use std::path::{Path, PathBuf};

use kawi_buffer::Buffer;
use kawi_editor::{Action, FoldMethod};

use super::*;
use crate::file_views::FileViewStore;
use crate::keymap::Flavor;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kawi_app_views_{}_{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_file(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path
}

fn open_app(path: &Path) -> App {
    App::new(Buffer::open(path.to_str().unwrap()).unwrap(), Flavor::Vim)
}

fn create_manual_fold(app: &mut App) {
    app.editor.apply(Action::JumpTo(1, 0));
    app.editor.apply(Action::EnterVisualLine);
    app.editor.apply(Action::JumpTo(3, 0));
    app.editor.apply(Action::FoldCreate);
}

fn store(dir: &Path) -> FileViewStore {
    FileViewStore::new(dir.join("views"))
}

#[test]
fn manual_folds_restore_for_direct_open() {
    let dir = temp_dir("manual_restore");
    let file = write_file(&dir, "alpha.txt", "one\ntwo\nthree\nfour\nfive\n");
    let store = store(&dir);
    let mut source = open_app(&file);
    create_manual_fold(&mut source);
    source.attach_file_views(store.clone());
    source.save_file_views().unwrap();
    let mut reopened = open_app(&file);
    reopened.attach_file_views(store);
    reopened.load_file_views();
    assert_eq!(reopened.editor.visible_rows(0, 10), vec![0, 1, 4]);
    assert_eq!(reopened.editor.fold_marker(1), '+');
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn changed_content_drops_manual_folds() {
    let dir = temp_dir("manual_changed");
    let file = write_file(&dir, "alpha.txt", "one\ntwo\nthree\nfour\nfive\n");
    let store = store(&dir);
    let mut source = open_app(&file);
    create_manual_fold(&mut source);
    source.attach_file_views(store.clone());
    source.save_file_views().unwrap();
    fs::write(&file, "one\ntwo\nthree\nfour\nfive\nsix\n").unwrap();
    let mut reopened = open_app(&file);
    reopened.attach_file_views(store);
    reopened.load_file_views();
    assert_eq!(reopened.editor.visible_rows(0, 10), vec![0, 1, 2, 3, 4, 5]);
    assert_eq!(reopened.editor.fold_marker(1), ' ');
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn indent_folds_restore_with_the_current_settings() {
    let dir = temp_dir("indent_restore");
    let file = write_file(
        &dir,
        "alpha.py",
        "def main():\n    a = 1\n    b = 2\nd = 4\n",
    );
    let store = store(&dir);
    let mut source = open_app(&file);
    source.editor.set_shiftwidth(4);
    source.editor.set_fold_method(FoldMethod::Indent);
    source.editor.close_all_folds();
    source.attach_file_views(store.clone());
    source.save_file_views().unwrap();
    let mut reopened = open_app(&file);
    reopened.editor.set_shiftwidth(4);
    reopened.editor.set_fold_method(FoldMethod::Indent);
    reopened.attach_file_views(store);
    reopened.load_file_views();
    assert_eq!(reopened.editor.visible_rows(0, 10), vec![0, 1, 3]);
    assert_eq!(reopened.editor.fold_marker(1), '+');
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn saving_without_folds_removes_the_view() {
    let dir = temp_dir("remove_empty");
    let file = write_file(&dir, "alpha.txt", "one\ntwo\nthree\nfour\nfive\n");
    let store = store(&dir);
    let mut source = open_app(&file);
    create_manual_fold(&mut source);
    source.attach_file_views(store.clone());
    source.save_file_views().unwrap();
    let mut empty = open_app(&file);
    empty.attach_file_views(store.clone());
    empty.save_file_views().unwrap();
    assert_eq!(store.load(&file), None);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn loaded_views_are_not_applied_twice() {
    let dir = temp_dir("load_once");
    let file = write_file(&dir, "alpha.txt", "one\ntwo\nthree\nfour\nfive\n");
    let store = store(&dir);
    let mut source = open_app(&file);
    create_manual_fold(&mut source);
    source.attach_file_views(store.clone());
    source.save_file_views().unwrap();
    let mut reopened = open_app(&file);
    reopened.attach_file_views(store);
    reopened.load_file_views();
    reopened.editor.apply(Action::JumpTo(1, 0));
    reopened.editor.apply(Action::FoldOpen);
    reopened.load_file_views();
    assert_eq!(reopened.editor.visible_rows(0, 10), vec![0, 1, 2, 3, 4]);
    assert_eq!(reopened.editor.fold_marker(1), '-');
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn sync_loads_a_newly_opened_file() {
    let dir = temp_dir("sync_open");
    let file = write_file(&dir, "alpha.txt", "one\ntwo\nthree\nfour\nfive\n");
    let store = store(&dir);
    let mut source = open_app(&file);
    create_manual_fold(&mut source);
    source.attach_file_views(store.clone());
    source.save_file_views().unwrap();
    let mut reopened = App::new(Buffer::from_string(""), Flavor::Vim);
    reopened.attach_file_views(store);
    reopened
        .editor
        .edit_file(Some(file.to_str().unwrap()), false)
        .unwrap();
    reopened.sync_file_view();
    assert_eq!(reopened.editor.visible_rows(0, 10), vec![0, 1, 4]);
    assert_eq!(reopened.editor.fold_marker(1), '+');
    fs::remove_dir_all(dir).unwrap();
}
