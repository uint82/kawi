use std::fs;
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use kawi_buffer::Buffer;
use kawi_editor::Action;

use super::*;
use crate::app::save::SaveAfter;
use crate::ipc::WaitOutcome;
use crate::keymap::Flavor;

fn git(dir: &std::path::Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?} failed");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn repo_with_staged_change(name: &str) -> (std::path::PathBuf, App) {
    let dir = crate::test_support::temp_dir(name);
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "Test"]);
    let path = dir.join("a.txt");
    fs::write(&path, "one\n").unwrap();
    git(&dir, &["add", "a.txt"]);
    git(&dir, &["commit", "-qm", "initial"]);
    fs::write(&path, "two\n").unwrap();
    git(&dir, &["add", "a.txt"]);
    let app = App::new(
        Buffer::from_file(path.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    (dir, app)
}

fn wait_for_status(app: &mut App) {
    app.refresh_git_status();
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.git_status.status().is_none() {
        assert!(Instant::now() < deadline, "git status did not load");
        app.sync_git_status();
        thread::sleep(Duration::from_millis(5));
    }
}

fn insert_message(app: &mut App, message: &str) {
    app.editor.apply(Action::EnterInsert);
    for character in message.chars() {
        app.editor.apply(Action::InsertChar(character));
    }
    app.editor.apply(Action::ExitInsert);
}

fn wait_for_task(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.task.is_running() {
        assert!(Instant::now() < deadline, "task did not finish");
        app.sync_task();
        thread::sleep(Duration::from_millis(5));
    }
    app.sync_task();
}

fn open_socket_editor(
    app: &mut App,
    path: &std::path::Path,
) -> mpsc::Receiver<anyhow::Result<WaitOutcome>> {
    app.ensure_ipc();
    let root = app.task_root();
    let file = path.to_path_buf();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(crate::ipc::wait(&root, &file));
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.git_commit.editor.is_none() {
        assert!(Instant::now() < deadline, "git editor did not open");
        app.sync_ipc();
        thread::sleep(Duration::from_millis(5));
    }
    receiver
}

#[test]
fn socket_editor_saves_and_replies_ok() {
    let (dir, mut app) = repo_with_staged_change("socket_save");
    let path = dir.join(".git/COMMIT_EDITMSG");
    fs::write(&path, "").unwrap();
    let receiver = open_socket_editor(&mut app, &path);
    insert_message(&mut app, "socket commit");
    assert!(app.request_save(false, SaveAfter::None, true));
    app.close_active_window(false).unwrap();
    assert_eq!(
        receiver
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap(),
        WaitOutcome::Saved
    );
    assert_eq!(fs::read_to_string(&path).unwrap().trim(), "socket commit");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn socket_editor_cquit_replies_abort() {
    let (dir, mut app) = repo_with_staged_change("socket_abort");
    let path = dir.join(".git/COMMIT_EDITMSG");
    fs::write(&path, "").unwrap();
    let receiver = open_socket_editor(&mut app, &path);
    app.handle_cquit().unwrap();
    assert_eq!(
        receiver
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap(),
        WaitOutcome::Aborted
    );
    assert!(app.git_commit.editor.is_none());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn socket_editor_empty_message_replies_abort() {
    let (dir, mut app) = repo_with_staged_change("socket_empty");
    let path = dir.join(".git/COMMIT_EDITMSG");
    fs::write(&path, "").unwrap();
    let receiver = open_socket_editor(&mut app, &path);
    assert!(app.request_save(false, SaveAfter::None, true));
    app.close_active_window(false).unwrap();
    assert_eq!(
        receiver
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap(),
        WaitOutcome::Aborted
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn fallback_editor_commits_staged_changes() {
    let (dir, mut app) = repo_with_staged_change("fallback");
    app.open_fallback_editor(dir.clone(), vec!["commit".to_string()], TaskKind::Commit)
        .unwrap();
    insert_message(&mut app, "fallback commit");
    assert!(app.request_save(false, SaveAfter::None, true));
    app.close_active_window(false).unwrap();
    wait_for_task(&mut app);
    assert_eq!(git(&dir, &["log", "-1", "--format=%s"]), "fallback commit");
    assert_eq!(git(&dir, &["diff", "--cached", "--name-only"]), "");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn fallback_amend_changes_only_the_message() {
    let (dir, mut app) = repo_with_staged_change("fallback_amend");
    app.open_fallback_editor(
        dir.clone(),
        vec![
            "commit".to_string(),
            "--amend".to_string(),
            "--only".to_string(),
        ],
        TaskKind::Amend,
    )
    .unwrap();
    insert_message(&mut app, "amended message");
    assert!(app.request_save(false, SaveAfter::None, true));
    app.close_active_window(false).unwrap();
    wait_for_task(&mut app);
    assert_eq!(git(&dir, &["log", "-1", "--format=%s"]), "amended message");
    assert_eq!(git(&dir, &["show", "HEAD:a.txt"]), "one");
    assert_eq!(git(&dir, &["diff", "--cached", "--name-only"]), "a.txt");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn amend_on_an_unborn_branch_reports_it() {
    let dir = crate::test_support::temp_dir("amend_unborn");
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "Test"]);
    let path = dir.join("a.txt");
    fs::write(&path, "one\n").unwrap();
    let mut app = App::new(
        Buffer::from_file(path.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    wait_for_status(&mut app);
    app.start_git_amend(false).unwrap();
    assert_eq!(app.last_message(), Some("No commit to amend"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn amend_with_staged_changes_keeps_the_message() {
    let (dir, mut app) = repo_with_staged_change("amend");
    wait_for_status(&mut app);
    app.start_git_amend(true).unwrap();
    wait_for_task(&mut app);
    assert_eq!(git(&dir, &["log", "-1", "--format=%s"]), "initial");
    assert_eq!(git(&dir, &["diff", "--cached", "--name-only"]), "");
    assert_eq!(git(&dir, &["show", "HEAD:a.txt"]), "two");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn commit_without_staged_changes_reports_it() {
    let (dir, mut app) = repo_with_staged_change("nothing_staged");
    git(&dir, &["reset", "-q", "HEAD", "--", "a.txt"]);
    wait_for_status(&mut app);
    app.start_git_commit(false).unwrap();
    assert_eq!(
        app.last_message(),
        Some("Nothing staged; use :git commit -a")
    );
    let _ = fs::remove_dir_all(dir);
}
