use std::fs;
use std::path::PathBuf;

use kawi_buffer::Buffer;

fn case_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("kawi_insert_save_{}_{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create case dir");
    dir
}

#[test]
fn edits_after_saving_mid_insert_are_dirty_and_undoable() {
    let dir = case_dir("insert_save");
    let path = dir.join("file.txt");
    let mut buffer = Buffer::from_string("a");
    buffer.path = Some(path.to_string_lossy().into_owned());
    buffer.begin_change();
    buffer.maybe_checkpoint(0, 1);
    buffer.insert_char(0, 1, 'b');
    buffer.save(false).expect("save");
    assert!(!buffer.is_dirty());
    buffer.maybe_checkpoint(0, 2);
    buffer.insert_char(0, 2, 'c');
    assert!(buffer.is_dirty());
    buffer.undo(0, 2).expect("undo");
    assert_eq!(buffer.line(0).as_deref(), Some("ab"));
    assert!(!buffer.is_dirty());
    let _ = fs::remove_dir_all(dir);
}
