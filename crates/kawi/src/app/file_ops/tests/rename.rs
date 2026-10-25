use super::*;

#[test]
fn rename_moves_the_file_and_preserves_dirty_state() {
    let dir = temp_dir("rename");
    let source = dir.join("a.txt");
    let target = dir.join("b.txt");
    fs::write(&source, "alpha\n").unwrap();
    let mut app = app_for(&source);
    edit(&mut app, "x");
    command(&mut app, &format!("rename {}", target.display()));
    assert!(!source.exists());
    assert_eq!(fs::read_to_string(&target).unwrap(), "alpha\n");
    assert_eq!(app.editor.buffer_text(1).unwrap(), "xalpha\n");
    assert_eq!(app.editor.filename(), target.to_string_lossy());
    assert!(app.editor.is_dirty());
    assert!(message_text(&app).contains("Renamed file"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn rename_refuses_existing_targets_and_missing_parents() {
    let dir = temp_dir("rename_errors");
    let source = dir.join("a.txt");
    let exists = dir.join("exists.txt");
    fs::write(&source, "alpha\n").unwrap();
    fs::write(&exists, "old\n").unwrap();
    let mut app = app_for(&source);
    command(&mut app, &format!("rename {}", exists.display()));
    assert_eq!(
        app.last_message(),
        Some("E13: File exists (add ! to override)")
    );
    command(
        &mut app,
        &format!("rename {}", dir.join("missing/b.txt").display()),
    );
    assert!(
        message_text(&app).contains("parent directory"),
        "{}",
        message_text(&app)
    );
    assert!(source.exists());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn rename_moves_a_directory_and_rewrites_open_buffers() {
    let dir = temp_dir("rename_dir");
    let old = dir.join("old");
    let new = dir.join("new");
    fs::create_dir_all(old.join("sub")).unwrap();
    fs::write(old.join("a.txt"), "alpha\n").unwrap();
    fs::write(old.join("sub/b.txt"), "bravo\n").unwrap();
    let mut app = app_for(&old.join("a.txt"));
    app.editor
        .edit_file(Some(&old.join("sub/b.txt").to_string_lossy()), false)
        .unwrap();
    edit(&mut app, "x");
    command(
        &mut app,
        &format!("rename {} {}", old.display(), new.display()),
    );
    assert!(!old.exists());
    assert_eq!(fs::read_to_string(new.join("a.txt")).unwrap(), "alpha\n");
    assert_eq!(
        fs::read_to_string(new.join("sub/b.txt")).unwrap(),
        "bravo\n"
    );
    assert_eq!(app.editor.buffer_text(2).unwrap(), "xbravo\n");
    assert_eq!(
        app.editor.buffer_path(1),
        Some(new.join("a.txt").to_string_lossy().as_ref())
    );
    assert_eq!(
        app.editor.buffer_path(2),
        Some(new.join("sub/b.txt").to_string_lossy().as_ref())
    );
    assert!(app.editor.is_dirty());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn rename_moves_persisted_undo_to_the_new_path() {
    let dir = temp_dir("rename_undo");
    let source = dir.join("a.txt");
    let target = dir.join("b.txt");
    let store = dir.join("undo");
    fs::write(&source, "alpha\n").unwrap();
    let mut app = app_for(&source);
    app.undo.configure(store.clone());
    app.undo.set_exclusions(Vec::new());
    app.sync_undo_histories();
    edit(&mut app, "x");
    command(&mut app, "w");
    command(&mut app, &format!("rename {}", target.display()));
    drop(app);
    let mut restored = app_for(&target);
    restored.undo.configure(store);
    restored.undo.set_exclusions(Vec::new());
    restored.sync_undo_histories();
    restored.editor.apply(Action::Undo);
    assert_eq!(restored.editor.buffer_text(1).unwrap(), "alpha\n");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn permanent_delete_confirms_and_closes_the_buffer() {
    let dir = temp_dir("delete");
    let path = dir.join("a.txt");
    fs::write(&path, "alpha\n").unwrap();
    let mut app = app_for(&path);
    command(&mut app, "delete!");
    assert!(app.file_ops.is_some());
    app.handle_key(key(KeyCode::Char('y'))).expect("confirm");
    assert!(!path.exists());
    assert_eq!(app.editor.buffer_path(1), None);
    assert!(app.last_message().unwrap().contains("Deleted"));
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
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn paths_with_spaces_unicode_and_leading_dashes_round_trip() {
    let dir = temp_dir("unicode");
    let source = dir.join("na me ü.txt");
    let target = dir.join("-na me ü2.txt");
    fs::write(&source, "alpha\n").unwrap();
    let mut app = app_for(&source);
    command(
        &mut app,
        &format!("rename \"{}\" \"{}\"", source.display(), target.display()),
    );
    assert!(target.exists());
    assert_eq!(app.editor.filename(), target.to_string_lossy());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn rename_does_not_report_its_own_watcher_events() {
    let dir = temp_dir("rename_watch");
    let source = dir.join("a.txt");
    let target = dir.join("b.txt");
    fs::write(&source, "alpha\n").unwrap();
    let mut app = app_for(&source);
    app.sync_watched_files();
    command(&mut app, &format!("rename {}", target.display()));
    app.handle_disk_paths(&[source, target.clone()]);
    assert!(message_text(&app).contains("Renamed file"));
    assert!(app.disk_status(1).is_none());
    let _ = fs::remove_dir_all(dir);
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
fn rename_onto_itself_is_refused() {
    let dir = temp_dir("rename_self");
    let path = dir.join("a.txt");
    fs::write(&path, "alpha\n").unwrap();
    let mut app = app_for(&path);
    command(&mut app, &format!("rename {}", path.display()));
    assert_eq!(app.last_message(), Some("Old and new path are the same"));
    assert!(path.exists());
    let _ = fs::remove_dir_all(dir);
}
