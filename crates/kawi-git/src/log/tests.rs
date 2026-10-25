use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::{commit_files, commit_patch, file_diff, LogEntry, LogEvent, LogOptions, LogStream};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn repo(name: &str) -> PathBuf {
    let suffix = NEXT.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "kawi_log_{}_{}_{}",
        std::process::id(),
        name,
        suffix
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "Test"]);
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

fn commit(dir: &Path, name: &str, text: &str) -> String {
    fs::write(dir.join(name), text).unwrap();
    git(dir, &["add", name]);
    git(dir, &["commit", "-qm", text]);
    git(dir, &["rev-parse", "HEAD"])
}

fn collect(stream: LogStream) -> Vec<LogEntry> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut entries = Vec::new();
    while Instant::now() < deadline {
        match stream.try_recv() {
            Some(LogEvent::Entries(mut batch)) => entries.append(&mut batch),
            Some(LogEvent::Done(error)) => {
                assert!(error.is_none(), "log error: {error:?}");
                return entries;
            }
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    }
    panic!("log stream timed out");
}

fn start(root: &Path, options: LogOptions) -> LogStream {
    let (wake, _) = mpsc::channel();
    LogStream::start(root, &options, wake).unwrap()
}

#[test]
fn streams_commits_newest_first() {
    let dir = repo("stream");
    commit(&dir, "a.txt", "one");
    commit(&dir, "b.txt", "two");
    let entries = collect(start(&dir, LogOptions::default()));
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].subject, "two");
    assert_eq!(entries[1].subject, "one");
    assert_eq!(entries[0].author, "Test");
    assert!(entries[0].decorations.contains("HEAD -> main"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn filters_logs_and_follows_files() {
    let dir = repo("filters");
    commit(&dir, "a.txt", "first");
    commit(&dir, "b.txt", "second");
    let options = LogOptions {
        grep: Some("second".to_string()),
        ..LogOptions::default()
    };
    let entries = collect(start(&dir, options));
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].subject, "second");
    let options = LogOptions {
        path: Some(PathBuf::from("a.txt")),
        ..LogOptions::default()
    };
    let entries = collect(start(&dir, options));
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].subject, "first");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn lists_commit_files_for_root_rename_and_merge() {
    let dir = repo("files");
    let root = commit(&dir, "old.rs", "one\n");
    let root_files = commit_files(&dir, &root).unwrap();
    assert_eq!(root_files.len(), 1);
    assert_eq!(root_files[0].status, 'A');
    git(&dir, &["mv", "old.rs", "new.rs"]);
    git(&dir, &["commit", "-qm", "rename"]);
    let rename = git(&dir, &["rev-parse", "HEAD"]);
    let rename_files = commit_files(&dir, &rename).unwrap();
    assert_eq!(rename_files.len(), 1);
    assert_eq!(rename_files[0].status, 'R');
    assert_eq!(
        rename_files[0].old_path.as_deref(),
        Some(Path::new("old.rs"))
    );
    assert_eq!(rename_files[0].path, Path::new("new.rs"));
    git(&dir, &["checkout", "-qb", "side"]);
    commit(&dir, "side.txt", "side");
    git(&dir, &["checkout", "-q", "main"]);
    commit(&dir, "main.txt", "main");
    git(&dir, &["merge", "--no-ff", "-qm", "merge", "side"]);
    let merge = git(&dir, &["rev-parse", "HEAD"]);
    let merge_files = commit_files(&dir, &merge).unwrap();
    assert!(merge_files
        .iter()
        .any(|file| file.path == Path::new("side.txt")));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn builds_patch_and_file_diff_for_root_and_rename() {
    let dir = repo("diffs");
    let root = commit(&dir, "old.rs", "one\n");
    let patch = commit_patch(&dir, &root).unwrap();
    assert!(patch.iter().any(|line| line.contains("+one")));
    let root_files = commit_files(&dir, &root).unwrap();
    let diff = file_diff(&dir, &root, &root_files[0]).unwrap();
    assert!(diff.iter().any(|line| line.contains("+one")));
    git(&dir, &["mv", "old.rs", "new.rs"]);
    git(&dir, &["commit", "-qm", "rename"]);
    let sha = git(&dir, &["rev-parse", "HEAD"]);
    let files = commit_files(&dir, &sha).unwrap();
    let diff = file_diff(&dir, &sha, &files[0]).unwrap();
    assert!(diff
        .iter()
        .any(|line| line.starts_with("rename from old.rs")));
    let _ = fs::remove_dir_all(dir);
}
