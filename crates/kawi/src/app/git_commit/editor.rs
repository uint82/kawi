use std::path::PathBuf;

use super::{EditorReply, FallbackCommit, GitEditorSession, TaskKind};
use crate::app::App;
use crate::ipc::{Connection, Event, WaitOutcome};

impl App {
    pub(super) fn handle_ipc_event(&mut self, event: Event) -> bool {
        match event {
            Event::OpenWait { path, connection } => {
                self.open_socket_editor(path, connection);
                true
            }
        }
    }

    fn open_socket_editor(&mut self, path: PathBuf, connection: Connection) {
        if self.git_commit.editor.is_some() {
            let _ = connection.reply(WaitOutcome::Aborted);
            self.show_message("Git editor is already open".to_string());
            return;
        }
        if self.open_editor_buffer(&path).is_err() {
            let _ = connection.reply(WaitOutcome::Aborted);
            return;
        }
        let buffer = self.editor.buffer_number();
        self.git_commit.editor = Some(GitEditorSession {
            buffer,
            written: false,
            reply: EditorReply::Socket(connection),
            path,
        });
        self.show_message("Git: :wq commits, :cq aborts".to_string());
    }

    pub(super) fn open_fallback_editor(
        &mut self,
        root: PathBuf,
        args: Vec<String>,
        kind: TaskKind,
    ) -> anyhow::Result<()> {
        let Some(git_dir) = kawi_git::git_dir(&root)? else {
            anyhow::bail!("cannot resolve git dir");
        };
        let path = git_dir.join("COMMIT_EDITMSG");
        std::fs::write(&path, b"")?;
        if let Err(error) = self.open_editor_buffer(&path) {
            self.show_message(format!("Git editor failed: {error}"));
            return Ok(());
        }
        let buffer = self.editor.buffer_number();
        self.git_commit.editor = Some(GitEditorSession {
            buffer,
            written: false,
            reply: EditorReply::Fallback(FallbackCommit { root, args, kind }),
            path,
        });
        self.show_message("Git: :wq commits, :cq aborts".to_string());
        Ok(())
    }

    fn open_editor_buffer(&mut self, path: &std::path::Path) -> anyhow::Result<()> {
        let name = path.to_string_lossy().into_owned();
        self.new_window(false, Some(name))?;
        let opened = self
            .editor
            .buffer_path(self.editor.buffer_number())
            .map(std::path::PathBuf::from);
        if opened.as_deref() != Some(path) {
            anyhow::bail!("cannot open {}", path.display());
        }
        Ok(())
    }

    pub(crate) fn finish_git_editor_for_buffer(&mut self, buffer: usize) {
        let Some(editor) = self.git_commit.editor.as_ref() else {
            return;
        };
        if editor.buffer != buffer {
            return;
        }
        let clean = !self.editor.buffer_is_dirty(buffer).unwrap_or(true);
        let text = self.editor.buffer_text(buffer).unwrap_or_default();
        let saved = self
            .git_commit
            .editor
            .as_ref()
            .is_some_and(|editor| editor.written)
            && clean
            && !text.trim().is_empty();
        self.finish_git_editor(saved);
    }

    pub(super) fn finish_git_editor(&mut self, saved: bool) {
        let Some(editor) = self.git_commit.editor.take() else {
            return;
        };
        match editor.reply {
            EditorReply::Socket(connection) => {
                let outcome = if saved {
                    WaitOutcome::Saved
                } else {
                    WaitOutcome::Aborted
                };
                let _ = connection.reply(outcome);
            }
            EditorReply::Fallback(fallback) => {
                if saved {
                    self.start_fallback_commit(fallback, editor.path);
                }
            }
        }
    }

    fn start_fallback_commit(&mut self, fallback: FallbackCommit, path: PathBuf) {
        let mut args = fallback.args;
        args.push("-F".to_string());
        args.push(path.to_string_lossy().into_owned());
        args.push("--cleanup=strip".to_string());
        let parser = super::commit_parser(&fallback.root);
        if !self.start_git_task(super::GitTask {
            name: "git commit".to_string(),
            args,
            env: Default::default(),
            detached: false,
            kind: fallback.kind,
            parser,
            root: fallback.root,
        }) {
            self.show_message("Cannot start git commit".to_string());
        }
    }

    pub(crate) fn abort_git_editor(&mut self) {
        let Some(editor) = self.git_commit.editor.take() else {
            return;
        };
        if let EditorReply::Socket(connection) = editor.reply {
            let _ = connection.reply(WaitOutcome::Aborted);
        }
    }

    pub(crate) fn handle_cquit(&mut self) -> anyhow::Result<bool> {
        let Some(buffer) = self.git_commit.editor_buffer() else {
            self.show_message("E: No git editor session".to_string());
            return Ok(false);
        };
        self.finish_git_editor_for_buffer(buffer);
        if self.windows.len() > 1 {
            self.close_active_window(true)?;
        }
        Ok(false)
    }
}
