mod editor;
mod push;
mod tasks;

use std::path::PathBuf;

use kawi_git::Status;
use kawi_task::ParserSpec;

use super::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TaskKind {
    Commit,
    Amend,
    Push,
    Fetch,
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
    pending_push: Option<push::PendingPush>,
    amend_warning: bool,
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

    pub(super) fn has_pending_push(&self) -> bool {
        self.pending_push.is_some()
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

    pub(super) fn start_git_fetch(&mut self) -> anyhow::Result<bool> {
        let Some(status) = self.git_status.status().cloned() else {
            self.show_message("No git repository".to_string());
            return Ok(false);
        };
        let mut env = std::collections::BTreeMap::new();
        env.insert("GIT_TERMINAL_PROMPT".to_string(), "0".to_string());
        self.start_git_task(GitTask {
            name: "git fetch".to_string(),
            args: vec!["fetch".to_string()],
            env,
            detached: true,
            kind: TaskKind::Fetch,
            parser: ParserSpec::None,
            root: status.root,
        });
        Ok(false)
    }

    fn start_editor_commit(
        &mut self,
        status: &Status,
        args: Vec<String>,
        kind: TaskKind,
    ) -> anyhow::Result<bool> {
        if kind != TaskKind::Amend {
            self.git_commit.amend_warning = false;
        }
        self.ensure_ipc();
        if self.ipc.is_some() {
            let editor = crate::ipc::editor_command()?;
            let mut env = std::collections::BTreeMap::new();
            env.insert("GIT_EDITOR".to_string(), editor);
            if let Some(socket) = self.ipc.as_ref().map(crate::ipc::Server::path) {
                env.insert(
                    crate::ipc::SOCKET_ENV.to_string(),
                    socket.to_string_lossy().into_owned(),
                );
            }
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

    fn warn_pushed_amend(&mut self, status: &Status) {
        self.git_commit.amend_warning = status
            .upstream()
            .is_some_and(|upstream| upstream.ahead == 0);
        if self.git_commit.amend_warning {
            self.show_message(
                "HEAD is already on upstream; amending requires a force push".to_string(),
            );
        }
    }

    pub(super) fn ensure_ipc(&mut self) {
        let root = self.project_root();
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

fn commit_parser(root: &std::path::Path) -> ParserSpec {
    if root.join("Cargo.toml").is_file() {
        ParserSpec::Cargo
    } else {
        ParserSpec::Gcc
    }
}

#[cfg(test)]
mod tests;
