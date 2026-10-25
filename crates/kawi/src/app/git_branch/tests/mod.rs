use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;
use kawi_editor::Action;
use kawi_picker::PickerKind;

use super::super::App;
use crate::keymap::Flavor;

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn repo(name: &str) -> PathBuf {
    let suffix = NEXT.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "kawi_git_branch_app_{}_{}_{}",
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

fn app(path: &Path) -> App {
    App::new(
        Buffer::from_file(path.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    )
}

fn wait_log(app: &mut App) -> usize {
    wait_for_log_rows(app, |rows| rows > 0)
}

fn wait_log_growth(app: &mut App, previous: usize) -> usize {
    wait_for_log_rows(app, |rows| rows > previous)
}

fn wait_for_log_rows(app: &mut App, enough: impl Fn(usize) -> bool) -> usize {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        app.refresh_picker();
        let rows = log_rows(app);
        if enough(rows) {
            return rows;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("log picker row loading timed out");
}

fn log_rows(app: &App) -> usize {
    app.picker
        .as_ref()
        .map_or(0, |session| session.picker().rows().len())
}

mod log;

#[test]
fn branch_picker_creates_from_the_selected_entry() {
    let dir = repo("prompt");
    let path = dir.join("a.txt");
    commit(&dir, "a.txt", "one\n");
    git(&dir, &["branch", "base"]);
    let mut app = app(&path);
    app.open_branch_picker().unwrap();
    app.handle_picker_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL))
        .unwrap();
    assert!(app.branch_prompt.is_some());
    for character in "from-base".chars() {
        app.handle_branch_prompt_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE))
            .unwrap();
    }
    app.handle_branch_prompt_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    assert!(app.branch_prompt.is_none());
    assert!(git(&dir, &["branch", "--list", "from-base"]).contains("from-base"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn detached_head_is_pinned_at_the_top() {
    let dir = repo("detached");
    let path = dir.join("a.txt");
    commit(&dir, "a.txt", "one\n");
    git(&dir, &["checkout", "-q", "--detach", "HEAD"]);
    let mut app = app(&path);
    app.open_branch_picker().unwrap();
    let rows = app.picker.as_ref().unwrap().picker().rows();
    assert!(rows[0].display.contains("detached at"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn non_repository_commands_are_disabled() {
    let dir = std::env::temp_dir().join(format!(
        "kawi_git_branch_nonrepo_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("a.txt");
    fs::write(&path, "one\n").unwrap();
    let mut app = app(&path);
    app.open_branch_picker().unwrap();
    assert!(app.picker.is_none());
    assert!(app
        .message
        .as_deref()
        .is_some_and(|message| message.contains("Not inside")));
    app.open_log_picker(false, None, None).unwrap();
    assert!(app.picker.is_none());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn branch_picker_switches_and_reloads_the_buffer() {
    let dir = repo("switch");
    let path = dir.join("a.txt");
    commit(&dir, "a.txt", "main\n");
    git(&dir, &["checkout", "-qb", "other"]);
    commit(&dir, "a.txt", "other\n");
    git(&dir, &["checkout", "-q", "main"]);
    let mut app = app(&path);
    app.open_branch_picker().unwrap();
    assert_eq!(
        app.picker.as_ref().unwrap().picker().kind(),
        PickerKind::GitBranches
    );
    let full_ref = app
        .git_branch
        .branches
        .as_ref()
        .unwrap()
        .branches
        .iter()
        .find(|branch| branch.name == "other")
        .unwrap()
        .full_ref
        .clone();
    app.switch_selected_branch(&full_ref).unwrap();
    assert_eq!(git(&dir, &["branch", "--show-current"]), "other");
    assert_eq!(app.editor.text_lines(), vec!["other".to_string()]);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn branch_refusal_offers_stash_and_switch() {
    let dir = repo("stash");
    let path = dir.join("a.txt");
    commit(&dir, "a.txt", "one\n");
    git(&dir, &["branch", "other"]);
    fs::write(&path, "two\n").unwrap();
    git(&dir, &["commit", "-qam", "two"]);
    git(&dir, &["checkout", "-q", "other"]);
    fs::write(&path, "three\n").unwrap();
    let mut app = app(&path);
    app.open_branch_picker().unwrap();
    let full_ref = app
        .git_branch
        .branches
        .as_ref()
        .unwrap()
        .branches
        .iter()
        .find(|branch| branch.name == "main")
        .unwrap()
        .full_ref
        .clone();
    app.switch_selected_branch(&full_ref).unwrap();
    assert!(app.git_branch.pending_switch.is_some());
    app.handle_pending_branch_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE))
        .unwrap();
    assert_eq!(git(&dir, &["branch", "--show-current"]), "main");
    assert!(app.git_branch.pending_switch.is_none());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn creates_the_first_branch_in_an_unborn_repository() {
    let dir = repo("unborn_create");
    let path = dir.join("a.txt");
    fs::write(&path, "one\n").unwrap();
    let mut app = app(&path);
    app.create_branch("first", None).unwrap();
    assert_eq!(git(&dir, &["symbolic-ref", "HEAD"]), "refs/heads/first");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn creates_and_deletes_branches() {
    let dir = repo("create");
    let path = dir.join("a.txt");
    commit(&dir, "a.txt", "one\n");
    let mut app = app(&path);
    app.create_branch("feat/new", None).unwrap();
    assert!(git(&dir, &["branch", "--list", "feat/new"]).contains("feat/new"));
    app.delete_branch("feat/new", false).unwrap();
    assert!(git(&dir, &["branch", "--list", "feat/new"]).is_empty());
    let _ = fs::remove_dir_all(dir);
}
