use std::fs;

use super::*;
use crate::app::save::SaveAfter;

#[test]
fn fallback_commit_with_failing_hook_preserves_message() {
    let (dir, mut app) = repo_with_staged_change("hook_fail");
    install_hook(&dir, "exit 1");
    app.open_fallback_editor(dir.clone(), vec!["commit".to_string()], TaskKind::Commit)
        .unwrap();
    insert_message(&mut app, "must survive hook failure");
    assert!(app.request_save(false, SaveAfter::None, true));
    app.close_active_window(false).unwrap();
    wait_for_task(&mut app);
    assert_eq!(git(&dir, &["log", "-1", "--format=%s"]), "initial");
    assert_eq!(git(&dir, &["diff", "--cached", "--name-only"]), "a.txt");
    let message = fs::read_to_string(dir.join(".git/COMMIT_EDITMSG")).unwrap();
    assert!(message.contains("must survive hook failure"));
    assert_eq!(
        app.last_message(),
        Some("Commit failed; see the task output")
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn hook_modifying_staged_file_refreshes_status_and_reloads_buffer() {
    let (dir, mut app) = repo_with_staged_change("hook_modify");
    install_hook(&dir, "printf 'hook\\n' >> a.txt\ngit add a.txt");
    app.open_fallback_editor(dir.clone(), vec!["commit".to_string()], TaskKind::Commit)
        .unwrap();
    insert_message(&mut app, "hook commit");
    assert!(app.request_save(false, SaveAfter::None, true));
    app.close_active_window(false).unwrap();
    wait_for_task(&mut app);
    assert_eq!(git(&dir, &["log", "-1", "--format=%s"]), "hook commit");
    assert_eq!(git(&dir, &["show", "HEAD:a.txt"]), "two\nhook");
    assert_eq!(git(&dir, &["diff", "--cached", "--name-only"]), "");
    assert_eq!(app.editor.text_lines(), vec!["two", "hook"]);
    wait_for_clean_status(&mut app);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn amend_warning_requires_an_unpushed_head() {
    let (dir, mut app) = repo_with_staged_change("amend_warning");
    let status = kawi_git::Status {
        root: dir.clone(),
        git_dir: dir.join(".git"),
        branch: kawi_git::BranchStatus::Named {
            name: "main".to_string(),
            oid: Some("abc".to_string()),
            upstream: Some(kawi_git::Upstream {
                name: "origin/main".to_string(),
                ahead: 0,
                behind: 0,
            }),
        },
        operation: kawi_git::OperationState::None,
        entries: Vec::new(),
    };
    app.warn_pushed_amend(&status);
    assert!(app
        .last_message()
        .is_some_and(|message| message.contains("force push")));
    app.clear_message();
    let mut unpushed = status.clone();
    if let kawi_git::BranchStatus::Named { upstream, .. } = &mut unpushed.branch {
        upstream.as_mut().unwrap().ahead = 1;
    }
    app.warn_pushed_amend(&unpushed);
    assert_eq!(app.last_message(), None);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn git_commands_are_disabled_outside_a_repo() {
    let dir = crate::test_support::temp_dir("non_repo");
    let path = dir.join("a.txt");
    fs::write(&path, "one\n").unwrap();
    let mut app = App::new(
        Buffer::from_file(path.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    assert!(app.git_status.display().is_none());
    app.start_git_commit(false).unwrap();
    assert_eq!(app.last_message(), Some("No git repository"));
    app.start_git_amend(false).unwrap();
    assert_eq!(app.last_message(), Some("No git repository"));
    app.start_git_push(false).unwrap();
    assert_eq!(app.last_message(), Some("No git repository"));
    app.start_git_fetch().unwrap();
    assert_eq!(app.last_message(), Some("No git repository"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn failing_hook_prevents_editor_from_opening() {
    let (dir, mut app) = repo_with_staged_change("hook_fail_primary");
    install_hook(&dir, "exit 1");
    wait_for_status(&mut app);
    app.start_git_commit(false).unwrap();
    wait_for_task(&mut app);
    assert!(app.git_commit.editor.is_none());
    assert_eq!(git(&dir, &["log", "-1", "--format=%s"]), "initial");
    assert_eq!(git(&dir, &["diff", "--cached", "--name-only"]), "a.txt");
    assert!(app
        .last_message()
        .is_some_and(|message| message.contains("Commit failed")));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn amend_warning_survives_editor_open() {
    let (dir, mut app) = repo_with_staged_change("amend_warning_editor");
    let status = kawi_git::Status {
        root: dir.clone(),
        git_dir: dir.join(".git"),
        branch: kawi_git::BranchStatus::Named {
            name: "main".to_string(),
            oid: Some("abc".to_string()),
            upstream: Some(kawi_git::Upstream {
                name: "origin/main".to_string(),
                ahead: 0,
                behind: 0,
            }),
        },
        operation: kawi_git::OperationState::None,
        entries: Vec::new(),
    };
    app.warn_pushed_amend(&status);
    app.open_fallback_editor(dir.clone(), vec!["commit".to_string()], TaskKind::Commit)
        .unwrap();
    let message = app.last_message().unwrap();
    assert!(message.contains("force push"));
    assert!(message.contains(":wq commits"));
    let _ = fs::remove_dir_all(dir);
}
