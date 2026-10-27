use std::fs;

use super::*;

#[test]
fn session_restores_the_explorer() {
    let dir = temp_dir("restore_explorer");
    let source = write_file(&dir, "alpha.txt", "alpha\n");
    let mut app = empty_app();
    app.editor.edit_file(Some(&source), false).unwrap();
    app.explorer.set_enabled(true);
    app.explorer.open(dir.clone());
    app.explorer.select(dir.join("alpha.txt"));
    let session = app.session_snapshot().expect("snapshot");
    let restored_explorer = session.explorer.as_ref().expect("explorer");
    assert_eq!(
        restored_explorer.root.as_deref(),
        Some(dir.to_str().unwrap())
    );
    let mut restored = empty_app();
    restored.explorer.set_enabled(true);
    assert_eq!(restored.restore_session(&session), None);
    assert!(restored.explorer.is_visible());
    assert_eq!(restored.explorer.root(), Some(dir.as_path()));
    assert_eq!(
        restored
            .explorer
            .selected_row()
            .map(|row| row.path.as_path()),
        Some(dir.join("alpha.txt").as_path())
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn disabled_explorer_is_not_snapshotted() {
    let dir = temp_dir("disabled_explorer_snapshot");
    let source = write_file(&dir, "alpha.txt", "alpha\n");
    let mut app = empty_app();
    app.editor.edit_file(Some(&source), false).unwrap();
    let session = app.session_snapshot().expect("snapshot");
    assert!(session.explorer.is_none());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn disabled_explorer_ignores_a_restored_session() {
    let dir = temp_dir("disabled_explorer_restore");
    let source = write_file(&dir, "alpha.txt", "alpha\n");
    let mut source_app = empty_app();
    source_app.editor.edit_file(Some(&source), false).unwrap();
    source_app.explorer.set_enabled(true);
    source_app.explorer.open(dir.clone());
    let session = source_app.session_snapshot().expect("snapshot");
    let mut restored = empty_app();
    assert_eq!(restored.restore_session(&session), None);
    assert!(!restored.explorer.is_visible());
    assert!(!restored.explorer.is_enabled());
    fs::remove_dir_all(dir).unwrap();
}
