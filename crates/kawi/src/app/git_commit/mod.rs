mod editor;
mod tasks;

use std::path::PathBuf;

use kawi_git::Status;
use kawi_task::{ParserSpec, TaskOptions, TaskSpec, TaskStatus};

use super::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TaskKind {
    Commit,
    Amend,
}

pub(super) struct ActiveTask {
    kind: TaskKind,
}

pub(super) struct GitTask {
    pub name: String,
    pub args: Vec<String>,
    pub env: std::collections::BTreeMap<String, String>,
    pub detached: bool,
    pub kind: TaskKind,
    pub parser: ParserSpec,
    pub root: PathBuf,
}

pub(super) struct FallbackCommit {
    pub root: PathBuf,
    pub args: Vec<String>,
    pub kind: TaskKind,
}

pub(super) enum EditorReply {
    Socket(crate::ipc::Connection),
    Fallback(FallbackCommit),
}

pub(super) struct GitEditorSession {
    pub buffer: usize,
    pub written: bool,
    pub reply: EditorReply,
    pub path: PathBuf,
}

#[derive(Default)]
pub(super) struct GitCommitState {
    active: Option<ActiveTask>,
    pub editor: Option<GitEditorSession>,
}

impl GitCommitState {
    pub(super) fn take_active(&mut self) -> Option<TaskKind> {
        self.active.take().map(|task| task.kind)
    }

    pub(super) fn mark_saved(&mut self, buffer: usize) {
        if let Some(editor) = self
            .editor
            .as_mut()
            .filter(|editor| editor.buffer == buffer)
        {
            editor.written = true;
        }
    }

    pub(super) fn editor_buffer(&self) -> Option<usize> {
        self.editor.as_ref().map(|editor| editor.buffer)
    }
}

impl App {
    pub(super) fn start_git_commit(&mut self, all: bool) -> anyhow::Result<bool> {
        let Some(status) = self.git_status.status().cloned() else {
            self.show_message("No git repository".to_string());
            return Ok(false);
        };
        if status.has_conflicts() {
            self.show_message("Resolve conflicts before committing".to_string());
            return Ok(false);
        }
        if !all && !status.has_staged() {
            self.show_message("Nothing staged; use :git commit -a".to_string());
            return Ok(false);
        }
        let args = if all {
            vec!["commit".to_string(), "-a".to_string()]
        } else {
            vec!["commit".to_string()]
        };
        self.start_editor_commit(&status, args, TaskKind::Commit)
    }

    pub(super) fn start_git_amend(&mut self, staged: bool) -> anyhow::Result<bool> {
        let Some(status) = self.git_status.status().cloned() else {
            self.show_message("No git repository".to_string());
            return Ok(false);
        };
        if status.is_unborn() {
            self.show_message("No commit to amend".to_string());
            return Ok(false);
        }
        self.warn_pushed_amend(&status);
        if staged {
            if !status.has_staged() {
                self.show_message("Nothing staged; amend would not change the tree".to_string());
                return Ok(false);
            }
            let args = vec![
                "commit".to_string(),
                "--amend".to_string(),
                "--no-edit".to_string(),
            ];
            self.start_git_task(GitTask {
                name: "git amend".to_string(),
                args,
                env: Default::default(),
                detached: false,
                kind: TaskKind::Amend,
                parser: commit_parser(&status.root),
                root: status.root.clone(),
            });
            return Ok(false);
        }
        let args = vec![
            "commit".to_string(),
            "--amend".to_string(),
            "--only".to_string(),
        ];
        self.start_editor_commit(&status, args, TaskKind::Amend)
    }

    fn start_editor_commit(
        &mut self,
        status: &Status,
        args: Vec<String>,
        kind: TaskKind,
    ) -> anyhow::Result<bool> {
        self.ensure_ipc();
        if self.ipc.is_some() {
            let editor = crate::ipc::editor_command()?;
            let mut env = std::collections::BTreeMap::new();
            env.insert("GIT_EDITOR".to_string(), editor);
            self.start_git_task(GitTask {
                name: "git commit".to_string(),
                args,
                env,
                detached: false,
                kind,
                parser: commit_parser(&status.root),
                root: status.root.clone(),
            });
            return Ok(false);
        }
        self.open_fallback_editor(status.root.clone(), args, kind)?;
        Ok(false)
    }

    pub(super) fn start_git_task(&mut self, task: GitTask) -> bool {
        let spec = TaskSpec {
            name: task.name,
            command: "git".to_string(),
            args: task.args,
            cwd: None,
            env: task.env,
            parser: task.parser,
        };
        if !self.start_task_spec(
            spec,
            task.root,
            TaskOptions {
                detached: task.detached,
            },
        ) {
            return false;
        }
        self.git_commit.active = Some(ActiveTask { kind: task.kind });
        true
    }

    pub(super) fn finish_git_task(&mut self, status: TaskStatus) {
        let Some(kind) = self.git_commit.take_active() else {
            return;
        };
        if self.git_commit.editor.is_some() {
            self.abort_git_editor();
        }
        match kind {
            TaskKind::Commit | TaskKind::Amend => {
                let _ = self.handle_git_refresh();
                self.refresh_git_status();
            }
        }
        if !status.success && !status.cancelled {
            self.show_message(failure_message(kind));
        }
    }

    fn warn_pushed_amend(&mut self, status: &Status) {
        if status
            .upstream()
            .is_some_and(|upstream| upstream.ahead == 0)
        {
            self.show_message(
                "HEAD is already on upstream; amending requires a force push".to_string(),
            );
        }
    }

    pub(super) fn ensure_ipc(&mut self) {
        let root = self.task_root();
        if self.ipc.is_some() && self.ipc_root.as_deref() == Some(root.as_path()) {
            return;
        }
        self.ipc = None;
        self.ipc_root = None;
        match crate::ipc::Server::start(&root, self.events.wake()) {
            Ok(server) => {
                self.ipc = Some(server);
                self.ipc_root = Some(root);
            }
            Err(error) => {
                self.show_message(format!("Git editor socket unavailable: {error}"));
            }
        }
    }

    pub(super) fn sync_ipc(&mut self) -> bool {
        let mut events = Vec::new();
        if let Some(server) = self.ipc.as_ref() {
            while let Some(event) = server.try_recv() {
                events.push(event);
            }
        }
        let mut changed = false;
        for event in events {
            changed |= self.handle_ipc_event(event);
        }
        changed
    }

    pub(super) fn abort_git_editor_on_exit(&mut self) {
        self.abort_git_editor();
    }
}

fn failure_message(kind: TaskKind) -> String {
    match kind {
        TaskKind::Commit => "Commit failed; see the task output".to_string(),
        TaskKind::Amend => "Amend failed; see the task output".to_string(),
    }
}

fn commit_parser(root: &std::path::Path) -> ParserSpec {
    if root.join("Cargo.toml").is_file() {
        ParserSpec::Cargo
    } else {
        ParserSpec::Gcc
    }
}

#[cfg(test)]
mod tests {
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
}
