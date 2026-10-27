use super::*;

#[test]
fn permanent_delete_confirms_and_closes_the_buffer() {
    let dir = temp_dir("delete");
    let path = dir.join("a.txt");
    fs::write(&path, "alpha\n").unwrap();
    let mut app = app_for(&path);
    command(&mut app, "delete!");
    let prompt = app.file_ops_prompt().expect("prompt");
    assert!(prompt.label.contains("Permanently delete"));
    app.handle_key(key(KeyCode::Char('y'))).expect("confirm");
    assert!(!path.exists());
    assert_eq!(app.editor.buffer_path(1), None);
    assert!(app.last_message().unwrap().contains("Deleted"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn delete_removes_the_persisted_undo_entry() {
    use sha2::{Digest, Sha256};

    let dir = temp_dir("delete_undo");
    let path = dir.join("a.txt");
    let store = dir.join("undo");
    fs::write(&path, "alpha\n").unwrap();
    let mut app = app_for(&path);
    app.undo.configure(store.clone());
    app.undo.set_exclusions(Vec::new());
    app.sync_undo_histories();
    edit(&mut app, "x");
    command(&mut app, "w");
    let hash: [u8; 32] = Sha256::digest(fs::read(&path).unwrap()).into();
    let undo = kawi_buffer::UndoStore::new(store);
    assert!(undo.load(&path, &hash).unwrap().is_some());
    command(&mut app, "delete!");
    app.handle_key(key(KeyCode::Char('y'))).expect("confirm");
    assert!(!path.exists());
    assert!(undo.load(&path, &hash).unwrap().is_none());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn delete_keeps_dirty_buffers_as_unsaved_by_default() {
    let dir = temp_dir("delete_keep");
    let path = dir.join("a.txt");
    fs::write(&path, "alpha\n").unwrap();
    let mut app = app_for(&path);
    edit(&mut app, "x");
    command(&mut app, "delete!");
    app.handle_key(key(KeyCode::Char('y'))).expect("confirm");
    app.handle_key(key(KeyCode::Char('k'))).expect("keep");
    assert!(!path.exists());
    assert_eq!(app.editor.buffer_text(1).unwrap(), "xalpha\n");
    assert_eq!(app.editor.buffer_path(1), None);
    assert!(app.editor.is_dirty());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn delete_can_discard_dirty_buffers() {
    let dir = temp_dir("delete_discard");
    let path = dir.join("a.txt");
    fs::write(&path, "alpha\n").unwrap();
    let mut app = app_for(&path);
    edit(&mut app, "x");
    command(&mut app, "delete!");
    app.handle_key(key(KeyCode::Char('y'))).expect("confirm");
    app.handle_key(key(KeyCode::Char('d'))).expect("discard");
    assert!(!path.exists());
    let number = app.editor.buffer_number();
    assert_eq!(app.editor.buffer_text(number).unwrap(), "");
    assert_eq!(app.editor.buffer_path(number), None);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn delete_dirty_buffers_can_be_cancelled() {
    let dir = temp_dir("delete_cancel");
    let path = dir.join("a.txt");
    fs::write(&path, "alpha\n").unwrap();
    let mut app = app_for(&path);
    edit(&mut app, "x");
    command(&mut app, "delete!");
    app.handle_key(key(KeyCode::Char('y'))).expect("confirm");
    app.handle_key(key(KeyCode::Char('c'))).expect("cancel");
    assert!(path.exists());
    assert_eq!(
        app.editor.buffer_path(1),
        Some(path.to_string_lossy().as_ref())
    );
    assert_eq!(app.editor.buffer_text(1).unwrap(), "xalpha\n");
    assert!(app.editor.is_dirty());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn directory_delete_requires_typed_confirmation_and_closes_buffers() {
    let dir = temp_dir("delete_dir");
    let target = dir.join("old");
    fs::create_dir_all(target.join("sub")).unwrap();
    fs::write(target.join("a.txt"), "alpha\n").unwrap();
    fs::write(target.join("sub/b.txt"), "bravo\n").unwrap();
    let mut app = app_for(&target.join("a.txt"));
    app.editor
        .edit_file(Some(&target.join("sub/b.txt").to_string_lossy()), false)
        .unwrap();
    command(&mut app, &format!("delete! {}", target.display()));
    let prompt = app.file_ops_prompt().expect("prompt");
    assert!(prompt.label.contains("(3 items)"), "{}", prompt.label);
    assert!(prompt.label.contains("Type delete"), "{}", prompt.label);
    type_text(&mut app, "delete");
    app.handle_key(key(KeyCode::Enter)).expect("confirm");
    assert!(!target.exists());
    assert_eq!(app.editor.buffer_count(), 1);
    assert_eq!(app.editor.buffer_path(1), None);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn delete_refuses_protected_roots_without_force() {
    let dir = temp_dir("delete_protected");
    let path = dir.join("a.txt");
    fs::write(&path, "alpha\n").unwrap();
    let mut app = app_for(&path);
    command(&mut app, &format!("delete {}", dir.display()));
    assert!(message_text(&app).contains("Refusing to delete protected path"));
    assert!(dir.exists());
    let home = kawi_editor::paths::home_dir().expect("home");
    command(&mut app, &format!("delete {}", home.display()));
    assert!(message_text(&app).contains("Refusing to delete protected path"));
    let cwd = std::env::current_dir().expect("cwd");
    command(&mut app, &format!("delete {}", cwd.display()));
    assert!(message_text(&app).contains("Refusing to delete protected path"));
    assert!(app.file_ops_prompt().is_none());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn delete_allows_paths_outside_the_project_root() {
    let workspace = temp_dir("delete_outside_workspace");
    let outside = temp_dir("delete_outside_target");
    let current = workspace.join("current.txt");
    let target = outside.join("a.txt");
    fs::write(&current, "current\n").unwrap();
    fs::write(&target, "alpha\n").unwrap();
    let mut app = app_for(&current);
    command(&mut app, &format!("delete {}", target.display()));
    assert!(app.file_ops_prompt().is_some());
    app.handle_key(key(KeyCode::Esc)).expect("cancel");
    assert!(target.exists());
    let _ = fs::remove_dir_all(workspace);
    let _ = fs::remove_dir_all(outside);
}

#[test]
fn delete_does_not_report_its_own_watcher_events() {
    let dir = temp_dir("delete_watch");
    let path = dir.join("a.txt");
    fs::write(&path, "alpha\n").unwrap();
    let mut app = app_for(&path);
    app.sync_watched_files();
    command(&mut app, "delete!");
    app.handle_key(key(KeyCode::Char('y'))).expect("confirm");
    app.handle_disk_paths(std::slice::from_ref(&path));
    assert!(message_text(&app).contains("Deleted"));
    assert!(!message_text(&app).contains("Deleted on disk"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn delete_updates_an_open_explorer() {
    let dir = temp_dir("delete_explorer");
    let path = dir.join("a.txt");
    let other = dir.join("b.txt");
    fs::write(&path, "alpha\n").unwrap();
    fs::write(&other, "bravo\n").unwrap();
    let mut app = app_for(&path);
    app.explorer.set_enabled(true);
    app.explorer.open(dir.clone());
    app.explorer.select(path.clone());
    command(&mut app, &format!("delete! {}", path.display()));
    app.handle_key(key(KeyCode::Char('y'))).expect("confirm");
    assert!(!path.exists());
    assert!(app.explorer.rows().iter().all(|row| row.path != path));
    assert!(app.explorer.rows().iter().any(|row| row.path == other));
    let _ = fs::remove_dir_all(dir);
}
