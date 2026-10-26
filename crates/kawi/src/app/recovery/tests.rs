use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;
use std::fs;
use std::path::PathBuf;

use crate::keymap::Flavor;
use crate::recovery::{WorkspaceStore, SNAPSHOT_VERSION};

fn temp_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("kawi_app_recovery_{}_{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn command(app: &mut App, text: &str) {
    app.handle_key(key(KeyCode::Char(':'))).expect("command");
    for character in text.chars() {
        app.handle_key(key(KeyCode::Char(character)))
            .expect("command text");
    }
    app.handle_key(key(KeyCode::Enter)).expect("execute");
}

fn view() -> SessionView {
    SessionView {
        row: 0,
        col: 0,
        desired: 0,
        top: 0,
        leftcol: 0,
    }
}

fn file_snapshot(
    buffer_id: &str,
    path: &Path,
    text: &str,
    fingerprint: Option<Fingerprint>,
) -> Snapshot {
    Snapshot {
        header: SnapshotHeader {
            version: SNAPSHOT_VERSION,
            buffer_id: buffer_id.to_string(),
            name: path.to_string_lossy().into_owned(),
            path: Some(path.to_string_lossy().into_owned()),
            revision: 1,
            fingerprint,
            view: view(),
        },
        text: text.to_string(),
    }
}

fn write_file(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path
}

#[test]
fn changed_files_show_a_conflict_and_recover_explicitly() {
    let root = temp_dir("conflict");
    let cwd = root.clone();
    let path = write_file(&root, "alpha.txt", "disk\n");
    let fingerprint = Fingerprint::capture(&path);
    let store = WorkspaceStore::create(&root, &cwd).unwrap();
    store
        .save(file_snapshot("old-id", &path, "recovered\n", fingerprint))
        .unwrap();
    fs::write(&path, "newer\n").unwrap();
    let mut app = App::new(
        Buffer::from_file(path.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    assert_eq!(app.start_recovery(&root, &cwd, true), None);
    let lines = app.pager_lines().expect("prompt").join("\n");
    assert!(lines.contains("changed on disk"));
    app.handle_key(key(KeyCode::Char('r'))).expect("recover");
    assert_eq!(
        app.editor
            .buffer_text(app.editor.buffer_number())
            .as_deref(),
        Some("recovered\n")
    );
    assert!(app.editor.is_dirty());
    command(&mut app, "qa!");
    assert!(store.load_all().0.is_empty());
    assert!(store.directory().read_dir().unwrap().next().is_none());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn discarding_a_conflict_removes_only_that_snapshot() {
    let root = temp_dir("discard_conflict");
    let cwd = root.clone();
    let path = write_file(&root, "alpha.txt", "disk\n");
    let fingerprint = Fingerprint::capture(&path);
    let store = WorkspaceStore::create(&root, &cwd).unwrap();
    store
        .save(file_snapshot("old-id", &path, "recovered\n", fingerprint))
        .unwrap();
    fs::write(&path, "newer\n").unwrap();
    let mut app = App::new(
        Buffer::from_file(path.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    assert_eq!(app.start_recovery(&root, &cwd, true), None);
    app.handle_key(key(KeyCode::Char('d'))).expect("discard");
    assert_eq!(fs::read_to_string(&path).unwrap(), "newer\n");
    assert!(store.load_all().0.is_empty());
    command(&mut app, "qa!");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn skip_advances_to_the_next_snapshot_and_restores_input() {
    let root = temp_dir("skip_multiple");
    let cwd = root.clone();
    let alpha = write_file(&root, "alpha.txt", "alpha\nbeta\n");
    let bravo = write_file(&root, "bravo.txt", "bravo\n");
    let store = WorkspaceStore::create(&root, &cwd).unwrap();
    store
        .save(file_snapshot("alpha-id", &alpha, "alpha dirty\n", None))
        .unwrap();
    store
        .save(file_snapshot("bravo-id", &bravo, "bravo dirty\n", None))
        .unwrap();
    let mut app = App::new(
        Buffer::from_file(alpha.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    assert_eq!(app.start_recovery(&root, &cwd, false), None);
    assert!(app.pager_lines().is_some());
    app.handle_key(key(KeyCode::Char('s'))).expect("skip first");
    assert!(app.pager_lines().is_some());
    app.handle_key(key(KeyCode::Char('s')))
        .expect("skip second");
    assert!(app.pager_lines().is_none());
    assert_eq!(store.load_all().0.len(), 2);
    app.handle_key(key(KeyCode::Char('j'))).expect("move");
    assert_eq!(app.cursor_position(), (1, 0));
    command(&mut app, "qa!");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn enter_skips_the_snapshot_and_restores_input() {
    let root = temp_dir("enter_skip");
    let cwd = root.clone();
    let alpha = write_file(&root, "alpha.txt", "alpha\nbeta\n");
    let store = WorkspaceStore::create(&root, &cwd).unwrap();
    store
        .save(file_snapshot("alpha-id", &alpha, "alpha dirty\n", None))
        .unwrap();
    let mut app = App::new(
        Buffer::from_file(alpha.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    assert_eq!(app.start_recovery(&root, &cwd, false), None);
    app.handle_key(key(KeyCode::Enter)).expect("skip");
    assert!(app.pager_lines().is_none());
    assert!(app.recovery_prompt.is_none());
    assert_eq!(store.load_all().0.len(), 1);
    app.handle_key(key(KeyCode::Char('j'))).expect("move");
    assert_eq!(app.cursor_position(), (1, 0));
    command(&mut app, "qa!");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn file_argument_startup_only_offers_matching_snapshots() {
    let root = temp_dir("restricted");
    let cwd = root.clone();
    let alpha = write_file(&root, "alpha.txt", "alpha\n");
    let bravo = write_file(&root, "bravo.txt", "bravo\n");
    let store = WorkspaceStore::create(&root, &cwd).unwrap();
    store
        .save(file_snapshot("alpha-id", &alpha, "alpha dirty\n", None))
        .unwrap();
    store
        .save(file_snapshot("bravo-id", &bravo, "bravo dirty\n", None))
        .unwrap();
    let mut app = App::new(
        Buffer::from_file(alpha.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    assert_eq!(app.start_recovery(&root, &cwd, true), None);
    assert_eq!(
        app.recovery_prompt.as_ref().map(|prompt| prompt.total()),
        Some(1)
    );
    app.handle_key(key(KeyCode::Char('d'))).expect("discard");
    command(&mut app, "qa!");
    let remaining = store.load_all().0;
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].header.buffer_id, "bravo-id");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn abnormal_finish_writes_current_dirty_buffers() {
    let root = temp_dir("abnormal");
    let cwd = root.clone();
    let store = WorkspaceStore::create(&root, &cwd).unwrap();
    let mut app = App::new(Buffer::from_string("alpha\n"), Flavor::Vim);
    app.handle_key(key(KeyCode::Char('i'))).expect("insert");
    app.handle_key(key(KeyCode::Char('x'))).expect("type");
    app.handle_key(key(KeyCode::Esc)).expect("normal");
    assert_eq!(app.start_recovery(&root, &cwd, false), None);
    app.finish_recovery_abnormal();
    let snapshots = store.load_all().0;
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].text, "xalpha\n");
    assert!(store.directory().join("owner.lock").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn writing_a_recovered_buffer_clears_its_snapshot_on_clean_exit() {
    let root = temp_dir("saved_cleanup");
    let cwd = root.clone();
    let path = write_file(&root, "alpha.txt", "disk\n");
    let store = WorkspaceStore::create(&root, &cwd).unwrap();
    store
        .save(file_snapshot(
            "old-id",
            &path,
            "dirty\n",
            Fingerprint::capture(&path),
        ))
        .unwrap();
    let mut app = App::new(
        Buffer::from_file(path.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    assert_eq!(app.start_recovery(&root, &cwd, true), None);
    app.handle_key(key(KeyCode::Char('r'))).expect("recover");
    command(&mut app, "w");
    assert_eq!(fs::read_to_string(&path).unwrap(), "dirty\n");
    app.finish_recovery_normal();
    assert!(store.load_all().0.is_empty());
    assert!(store.directory().read_dir().unwrap().next().is_none());
    fs::remove_dir_all(root).unwrap();
}
