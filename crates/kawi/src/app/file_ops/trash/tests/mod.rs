mod freedesktop;
mod macos;

use super::TrashDir;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(1);

pub(super) fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "kawi_trash_{}_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
        name
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn trash_dir_delegates_to_the_backend() {
    let dir = temp_dir("delegate");
    let target = dir.join("a.txt");
    fs::write(&target, "alpha\n").unwrap();
    let trash = TrashDir::freedesktop(dir.join("Trash"));
    assert!(trash.can_trash(&target));
    let moved = trash.trash_as(&target, &target).unwrap();
    assert_eq!(fs::read_to_string(moved).unwrap(), "alpha\n");
    let _ = fs::remove_dir_all(dir);
}
