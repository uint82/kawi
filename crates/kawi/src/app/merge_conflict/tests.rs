use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;
use kawi_editor::Action;
use kawi_git::conflict::ConflictChoice;
use kawi_git::{OperationState, Status};
use kawi_tui::frame::Sign;

use super::*;
use crate::keymap::Flavor;
use crate::options::SignColumnMode;

const CONFLICT: &str = "alpha\n<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> topic\nomega\n";

fn app(text: &str) -> App {
    let mut app = App::new(Buffer::from_string(text), Flavor::Vim);
    app.handle_resize(80, 24).expect("resize");
    app.signcolumn = SignColumnMode::Auto;
    app.sync_merge_conflicts();
    app.update_signcolumn_width();
    app
}

fn occur(text: &str, line: usize) -> App {
    let mut app = app(text);
    app.editor.apply(Action::JumpTo(line, 0));
    app
}

#[test]
fn sync_places_conflict_signs_on_block_starts() {
    let app = app(CONFLICT);
    let signs = app.view_signs().expect("signs");
    assert_eq!(signs[1], Some(Sign::Conflict));
    assert!(signs.iter().filter(|sign| sign.is_some()).count() == 1);
}

#[test]
fn navigation_moves_between_conflict_blocks() {
    let mut app = app(&format!("{CONFLICT}{CONFLICT}"));
    app.handle_merge_conflict_navigation(true).expect("next");
    assert_eq!(app.editor.row(), 1);
    app.handle_merge_conflict_navigation(true).expect("next");
    assert_eq!(app.editor.row(), 8);
    app.handle_merge_conflict_navigation(false)
        .expect("previous");
    assert_eq!(app.editor.row(), 1);
}

#[test]
fn choosing_ours_replaces_the_whole_block() {
    let mut app = occur(CONFLICT, 2);
    app.handle_merge_conflict_choice(ConflictChoice::Ours)
        .expect("ours");
    assert_eq!(
        app.editor.visible_lines(0, 3),
        vec!["alpha", "ours", "omega"]
    );
    assert_eq!(app.last_message(), Some("Took ours"));
    assert!(app.merge_conflict.blocks().is_empty());
}

#[test]
fn choosing_theirs_and_both_replace_the_block() {
    let mut app = occur(CONFLICT, 3);
    app.handle_merge_conflict_choice(ConflictChoice::Theirs)
        .expect("theirs");
    assert_eq!(
        app.editor.visible_lines(0, 3),
        vec!["alpha", "theirs", "omega"]
    );

    let mut app = occur(CONFLICT, 4);
    app.handle_merge_conflict_choice(ConflictChoice::Both)
        .expect("both");
    assert_eq!(
        app.editor.visible_lines(0, 4),
        vec!["alpha", "ours", "theirs", "omega"]
    );
}

#[test]
fn resolution_preserves_a_missing_final_newline() {
    let mut app = occur("<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> topic", 1);
    app.handle_merge_conflict_choice(ConflictChoice::Ours)
        .expect("ours");
    assert_eq!(app.editor.text_lines(), vec!["ours"]);
    assert!(!app.editor.text_has_final_newline());
}

#[test]
fn choosing_outside_a_conflict_reports_an_error() {
    let mut app = occur(CONFLICT, 0);
    app.handle_merge_conflict_choice(ConflictChoice::Ours)
        .expect("ours");
    assert_eq!(app.last_message(), Some("No conflict at cursor"));
}

#[test]
fn continue_and_abort_map_operations_to_git_commands() {
    assert_eq!(
        continue_args(OperationState::Merge),
        Some(&["merge", "--continue"][..])
    );
    assert_eq!(
        continue_args(OperationState::Rebase),
        Some(&["rebase", "--continue"][..])
    );
    assert_eq!(
        continue_args(OperationState::CherryPick),
        Some(&["cherry-pick", "--continue"][..])
    );
    assert_eq!(
        continue_args(OperationState::Revert),
        Some(&["revert", "--continue"][..])
    );
    assert_eq!(continue_args(OperationState::Bisect), None);
    assert_eq!(continue_args(OperationState::None), None);
    assert_eq!(
        abort_args(OperationState::Merge),
        Some(&["merge", "--abort"][..])
    );
    assert_eq!(
        abort_args(OperationState::Bisect),
        Some(&["bisect", "reset"][..])
    );
    assert_eq!(abort_args(OperationState::None), None);
    assert_eq!(operation_name(OperationState::Merge), Some("merge"));
    assert_eq!(operation_name(OperationState::None), None);
}

#[test]
fn worktree_guard_rejects_unresolved_markers() {
    let dir = crate::test_support::temp_dir("merge_conflict_guard");
    let path = dir.join("a.txt");
    let app = App::new(Buffer::from_string(""), Flavor::Vim);
    fs::write(&path, CONFLICT).expect("write");
    assert_eq!(
        app.reject_worktree_conflict_markers(&path),
        Err("Resolve conflict markers before staging".to_string())
    );
    fs::write(&path, "resolved\n").expect("write");
    assert_eq!(app.reject_worktree_conflict_markers(&path), Ok(()));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn aborts_a_merge_after_confirmation() {
    let (dir, path) = conflicted_repo("merge_abort");
    let mut app = App::new(
        Buffer::from_file(path.to_str().expect("path")).expect("buffer"),
        Flavor::Vim,
    );
    wait_for_status(&mut app, |status| status.operation == OperationState::Merge);
    app.handle_git_continue().expect("continue");
    assert_eq!(
        app.last_message(),
        Some("Resolve conflicts before continuing")
    );
    app.handle_git_abort().expect("abort");
    assert_eq!(app.last_message(), Some("Abort merge? y/n"));
    app.handle_git_abort_confirmation(key(KeyCode::Char('y')))
        .expect("confirm");
    wait_for_task(&mut app);
    wait_for_status(&mut app, |status| {
        status.operation == OperationState::None && !status.has_conflicts()
    });
    assert_eq!(fs::read_to_string(&path).expect("content"), "main\n");
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn continue_completes_a_resolved_merge() {
    let (dir, path) = conflicted_repo("merge_continue");
    let mut app = App::new(
        Buffer::from_file(path.to_str().expect("path")).expect("buffer"),
        Flavor::Vim,
    );
    wait_for_status(&mut app, |status| status.operation == OperationState::Merge);
    app.editor.apply(Action::JumpTo(1, 0));
    app.handle_merge_conflict_choice(ConflictChoice::Ours)
        .expect("ours");
    app.handle_save(false).expect("save");
    kawi_git::stage_path(&dir, Path::new("a.txt")).expect("stage");
    wait_for_status(&mut app, |status| !status.has_conflicts());
    app.handle_git_continue().expect("continue");
    wait_for_task(&mut app);
    wait_for_status(&mut app, |status| status.operation == OperationState::None);
    let parents = git(&dir, &["log", "-1", "--format=%P"]);
    assert_eq!(parents.split_whitespace().count(), 2);
    fs::remove_dir_all(dir).expect("cleanup");
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .expect("git");
    assert!(output.status.success(), "git {args:?} failed");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn conflicted_repo(name: &str) -> (PathBuf, PathBuf) {
    let dir = crate::test_support::temp_dir(name);
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "Test"]);
    git(&dir, &["checkout", "-q", "-b", "main"]);
    let path = dir.join("a.txt");
    fs::write(&path, "base\n").expect("write");
    git(&dir, &["add", "a.txt"]);
    git(&dir, &["commit", "-qm", "base"]);
    git(&dir, &["checkout", "-q", "-b", "topic"]);
    fs::write(&path, "topic\n").expect("write");
    git(&dir, &["add", "a.txt"]);
    git(&dir, &["commit", "-qm", "topic"]);
    git(&dir, &["checkout", "-q", "main"]);
    fs::write(&path, "main\n").expect("write");
    git(&dir, &["add", "a.txt"]);
    git(&dir, &["commit", "-qm", "main"]);
    let output = Command::new("git")
        .current_dir(&dir)
        .args(["merge", "topic"])
        .output()
        .expect("merge");
    assert!(!output.status.success(), "merge should conflict");
    (dir, path)
}

fn wait_for_status(app: &mut App, predicate: impl Fn(&Status) -> bool) {
    app.refresh_git_status();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        app.sync_git_status();
        if app.git_status.status().is_some_and(&predicate) {
            return;
        }
        assert!(Instant::now() < deadline, "git status did not update");
        thread::sleep(Duration::from_millis(5));
    }
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
