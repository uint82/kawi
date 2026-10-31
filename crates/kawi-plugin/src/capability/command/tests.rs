use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use super::*;

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn temp_dir(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let id = NEXT.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("kawi_command_{name}_{stamp}_{id}"));
    fs::create_dir_all(&dir).expect("dir");
    dir
}

fn executable(path: &Path) {
    fs::write(path, "#!/bin/sh\n").expect("write");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("mode");
    }
}

fn canonical(path: &Path) -> String {
    fs::canonicalize(path)
        .expect("canonical")
        .to_string_lossy()
        .into_owned()
}

#[test]
fn validates_names_and_absolute_paths() {
    let absolute = std::env::current_exe().expect("current exe");
    assert!(validate("sh").is_ok());
    assert!(validate(absolute.to_str().expect("utf8")).is_ok());
    assert!(validate("").is_err());
    assert!(validate("   ").is_err());
    assert!(validate(".").is_err());
    assert!(validate("..").is_err());
    assert!(validate("./sh").is_err());
    assert!(validate("bin/sh").is_err());
}

#[test]
fn resolves_bare_names_to_absolute_paths() {
    assert!(resolve("sh").expect("sh").is_absolute());
}

#[test]
fn canonicalizes_absolute_paths() {
    let current = std::env::current_exe().expect("current exe");
    let resolved = resolve(current.to_str().expect("utf8")).expect("current exe");
    assert_eq!(resolved, fs::canonicalize(current).expect("canonical"));
}

#[test]
fn intersects_commands_by_canonical_path() {
    let absolute = resolve("sh").expect("sh").to_string_lossy().into_owned();
    assert_eq!(
        intersect(&["sh".to_string()], std::slice::from_ref(&absolute)),
        Some(vec![absolute.clone()])
    );
    assert_eq!(
        intersect(std::slice::from_ref(&absolute), &["sh".to_string()]),
        Some(vec![absolute])
    );
    assert_eq!(intersect(&["sh".to_string()], &["true".to_string()]), None);
}

#[test]
fn does_not_match_different_executables() {
    let first_dir = temp_dir("first");
    let second_dir = temp_dir("second");
    let first = first_dir.join("tool");
    let second = second_dir.join("tool");
    executable(&first);
    executable(&second);
    assert_eq!(
        intersect(
            &[first.to_string_lossy().into_owned()],
            &[second.to_string_lossy().into_owned()]
        ),
        None
    );
    fs::remove_dir_all(first_dir).expect("cleanup");
    fs::remove_dir_all(second_dir).expect("cleanup");
}

#[test]
#[cfg(unix)]
fn resolves_symlinks_to_the_same_canonical_path() {
    use std::os::unix::fs::symlink;

    let dir = temp_dir("symlink");
    let target = dir.join("tool");
    let link = dir.join("link");
    executable(&target);
    symlink(&target, &link).expect("symlink");
    assert_eq!(
        intersect(
            &[link.to_string_lossy().into_owned()],
            &[target.to_string_lossy().into_owned()]
        ),
        Some(vec![canonical(&target)])
    );
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn covers_requires_every_inner_command_to_resolve() {
    assert!(covers(&["sh".to_string()], &["sh".to_string()]));
    assert!(!covers(
        &["sh".to_string()],
        &["sh".to_string(), "/does/not/exist".to_string()]
    ));
    assert!(covers(
        &["sh".to_string(), "/does/not/exist".to_string()],
        &["sh".to_string()]
    ));
    assert!(!covers(
        &["/does/not/exist".to_string()],
        &["/does/not/exist".to_string()]
    ));
}
