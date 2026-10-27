use std::fs;

use super::*;

#[test]
fn session_restores_the_explorer() {
    let dir = temp_dir("restore_explorer");
    let source = write_file(&dir, "alpha.txt", "alpha\n");
    let mut app = empty_app();
    app.editor.edit_file(Some(&source), false).unwrap();
    app.explorer.open(dir.clone());
    app.explorer.select(dir.join("alpha.txt"));
    let session = app.session_snapshot().expect("snapshot");
    let restored_explorer = session.explorer.as_ref().expect("explorer");
    assert_eq!(
        restored_explorer.root.as_deref(),
        Some(dir.to_str().unwrap())
    );
    let mut restored = empty_app();
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
