use super::*;
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn temp_dir(name: &str) -> PathBuf {
    let suffix = NEXT.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "kawi_status_{}_{}_{}",
        std::process::id(),
        name,
        suffix
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?} failed");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn repo(name: &str) -> PathBuf {
    let dir = temp_dir(name);
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "Test"]);
    dir
}

fn parse(text: &str) -> Status {
    let mut bytes = Vec::new();
    for token in text.split('\0').filter(|token| !token.is_empty()) {
        bytes.extend_from_slice(token.as_bytes());
        bytes.push(0);
    }
    parse_status(&bytes).unwrap()
}

#[test]
fn parses_branch_and_ahead_behind() {
    let status = parse(
        "# branch.oid abcdef\0# branch.head main\0# branch.upstream origin/main\0# branch.ab +2 -3\0",
    );
    assert_eq!(status.branch_label(), "main");
    let upstream = status.upstream().unwrap();
    assert_eq!(upstream.ahead, 2);
    assert_eq!(upstream.behind, 3);
    assert!(!status.is_unborn());
    assert!(!status.is_detached());
}

#[test]
fn parses_detached_and_unborn_branches() {
    let detached = parse("# branch.oid abcdef\0# branch.head (detached)\0");
    assert!(detached.is_detached());
    assert_eq!(detached.branch_label(), "abcdef");
    let unborn = parse("# branch.oid (initial)\0# branch.head main\0");
    assert!(unborn.is_unborn());
    assert_eq!(unborn.branch_label(), "main");
}

#[test]
fn parses_ordinary_renamed_unmerged_and_untracked_entries() {
    let status = parse(
        "1 M. N... 100644 100644 100644 a b src/main.rs\0\
         2 R. N... 100644 100644 100644 a b R100 new name.rs\0old name.rs\0\
         u UU N... 100644 100644 100644 100644 a b c conflict.rs\0\
         ? untracked file.txt\0",
    );
    assert_eq!(status.entries.len(), 4);
    assert_eq!(status.entries[0].path, PathBuf::from("src/main.rs"));
    assert_eq!(status.entries[0].index, StatusCode::Modified);
    assert_eq!(
        status.entries[1].original_path.as_deref(),
        Some(Path::new("old name.rs"))
    );
    assert_eq!(status.entries[1].path, PathBuf::from("new name.rs"));
    assert!(status.entries[2].is_conflict());
    assert_eq!(status.entries[3].kind, EntryKind::Untracked);
    assert_eq!(status.entries[3].path, PathBuf::from("untracked file.txt"));
}

#[test]
fn tracks_both_staged_and_unstaged_changes() {
    let status = parse("1 MM N... 100644 100644 100644 a b a.rs\0");
    let entry = &status.entries[0];
    assert!(entry.has_staged());
    assert!(entry.has_unstaged());
    assert_eq!(entry.code(), "MM");
    assert!(status.has_staged());
    assert!(!status.has_conflicts());
}

#[test]
fn loads_a_real_repository() {
    let dir = repo("real");
    fs::write(dir.join("tracked.txt"), "one\n").unwrap();
    git(&dir, &["add", "tracked.txt"]);
    git(&dir, &["commit", "-qm", "one"]);
    fs::write(dir.join("tracked.txt"), "two\n").unwrap();
    fs::write(dir.join("staged file.txt"), "new\n").unwrap();
    git(&dir, &["add", "staged file.txt"]);
    fs::write(dir.join("untracked file.txt"), "new\n").unwrap();
    let status = Status::load(&dir).unwrap().unwrap();
    assert_eq!(
        status.branch_label(),
        git(&dir, &["branch", "--show-current"])
    );
    assert!(status.has_staged());
    assert!(status.entries.iter().any(|entry| {
        entry.path == Path::new("untracked file.txt") && entry.kind == EntryKind::Untracked
    }));
    assert!(status.entries.iter().any(|entry| {
        entry.path == Path::new("staged file.txt") && entry.index == StatusCode::Added
    }));
    assert!(status.entries.iter().any(|entry| {
        entry.path == Path::new("tracked.txt") && entry.worktree == StatusCode::Modified
    }));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn loads_from_a_file_inside_a_repository() {
    let dir = repo("file");
    let nested = dir.join("nested");
    fs::create_dir(&nested).unwrap();
    let file = nested.join("a.txt");
    fs::write(&file, "one\n").unwrap();
    let status = Status::load(&file).unwrap().unwrap();
    assert_eq!(status.root, fs::canonicalize(&dir).unwrap());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn loads_unborn_and_empty_repository() {
    let dir = repo("unborn");
    let status = Status::load(&dir).unwrap().unwrap();
    assert!(status.is_unborn());
    assert!(!status.is_detached());
    assert!(status.entries.is_empty());
    assert_eq!(
        status.branch_label(),
        git(&dir, &["branch", "--show-current"])
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn missing_and_bare_repositories_are_disabled() {
    let missing = temp_dir("missing");
    assert!(Status::load(&missing).unwrap().is_none());
    let bare = temp_dir("bare");
    git(&bare, &["init", "--bare", "-q"]);
    assert!(Status::load(&bare).unwrap().is_none());
    let _ = fs::remove_dir_all(missing);
    let _ = fs::remove_dir_all(bare);
}

#[test]
fn detects_rebase_and_merge_state() {
    let dir = repo("operation");
    let status = Status::load(&dir).unwrap().unwrap();
    fs::create_dir_all(status.git_dir.join("rebase-merge")).unwrap();
    assert_eq!(
        Status::load(&dir).unwrap().unwrap().operation,
        OperationState::Rebase
    );
    fs::remove_dir_all(status.git_dir.join("rebase-merge")).unwrap();
    fs::write(status.git_dir.join("MERGE_HEAD"), "deadbeef\n").unwrap();
    assert_eq!(
        Status::load(&dir).unwrap().unwrap().operation,
        OperationState::Merge
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn loads_a_worktree_where_git_is_a_file() {
    let dir = repo("worktree");
    fs::write(dir.join("a.txt"), "one\n").unwrap();
    git(&dir, &["add", "a.txt"]);
    git(&dir, &["commit", "-qm", "one"]);
    let worktree = dir.join("linked");
    git(&dir, &["worktree", "add", "-q", worktree.to_str().unwrap()]);
    assert!(worktree.join(".git").is_file());
    let status = Status::load(&worktree).unwrap().unwrap();
    assert!(status.git_dir.join("HEAD").is_file());
    assert_eq!(
        status.branch_label(),
        git(&worktree, &["branch", "--show-current"])
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
#[cfg(all(unix, not(target_os = "macos")))]
// TODO: enable on macOS when the test can place non-UTF-8 bytes in the Git index.
fn preserves_non_utf8_paths() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    let dir = repo("non_utf8");
    let name = OsString::from_vec(vec![b'a', 0xff, b'b']);
    let path = dir.join(&name);
    fs::write(&path, "data\n").unwrap();
    let status = Status::load(&dir).unwrap().unwrap();
    let entry = status
        .entries
        .iter()
        .find(|entry| entry.kind == EntryKind::Untracked)
        .unwrap();
    assert_eq!(entry.path.as_os_str(), name.as_os_str());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn status_refresh_does_not_collide_with_commit() {
    let dir = repo("concurrent");
    fs::write(dir.join("a.txt"), "one\n").unwrap();
    git(&dir, &["add", "a.txt"]);
    git(&dir, &["commit", "-qm", "one"]);
    fs::write(dir.join("a.txt"), "two\n").unwrap();
    git(&dir, &["add", "a.txt"]);
    let reader_dir = dir.clone();
    let reader = thread::spawn(move || {
        for _ in 0..50 {
            assert!(Status::load(&reader_dir).unwrap().is_some());
        }
    });
    let output = Command::new("git")
        .current_dir(&dir)
        .args(["commit", "-m", "two"])
        .env("GIT_EDITOR", "true")
        .output()
        .unwrap();
    reader.join().unwrap();
    assert!(output.status.success(), "commit failed: {output:?}");
    assert_eq!(git(&dir, &["log", "-1", "--format=%s"]), "two");
    let _ = fs::remove_dir_all(dir);
}
