mod delete;
mod fs;
mod pending;
mod prompt;
mod rename;
mod trash;
mod write;

pub(crate) use pending::PendingFileOp;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use crossterm::event::KeyEvent;
use kawi_editor::paths;

use super::App;
pub(super) use prompt::FileOpSession;
use prompt::{ConfirmAction, ConfirmOutcome, DeleteOutcome, PathAction, PathOutcome};

impl App {
    pub(super) fn open_rename_prompt(&mut self, source: PathBuf, force: bool) {
        self.file_ops = Some(FileOpSession::Path(prompt::PathPrompt::rename(
            source, force,
        )));
    }

    pub(super) fn open_new_file_prompt(&mut self, root: PathBuf) {
        self.file_ops = Some(FileOpSession::Path(prompt::PathPrompt::new_file(root)));
    }

    pub(super) fn open_new_directory_prompt(&mut self, root: PathBuf) {
        self.file_ops = Some(FileOpSession::Path(prompt::PathPrompt::new_directory(root)));
    }

    pub(super) fn handle_file_op_key(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        let Some(mut session) = self.file_ops.take() else {
            return Ok(false);
        };
        let keep = match &mut session {
            FileOpSession::Path(prompt) => match prompt.handle_key(key) {
                PathOutcome::Consumed => true,
                PathOutcome::Cancel => false,
                PathOutcome::Submit { action, text } => {
                    self.submit_path_action(action, &text);
                    false
                }
            },
            FileOpSession::Confirm(prompt) => match prompt.handle_key(key) {
                ConfirmOutcome::Consumed => true,
                ConfirmOutcome::Cancel => false,
                ConfirmOutcome::Accept(ConfirmAction::CreateParents(pending)) => {
                    self.create_parents_and_write(pending);
                    false
                }
            },
            FileOpSession::Delete(prompt) => match prompt.handle_key(key) {
                DeleteOutcome::Consumed => true,
                DeleteOutcome::Cancel => false,
                DeleteOutcome::Execute { keep_dirty } => {
                    let request = prompt.request().clone();
                    self.begin_delete(request, keep_dirty);
                    false
                }
            },
        };
        if keep {
            self.file_ops = Some(session);
        }
        Ok(false)
    }

    pub(super) fn paste_file_op(&mut self, text: &str) -> bool {
        let Some(session) = self.file_ops.as_mut() else {
            return false;
        };
        match session {
            FileOpSession::Path(prompt) => prompt.paste(text),
            FileOpSession::Delete(prompt) => prompt.paste(text),
            FileOpSession::Confirm(_) => return true,
        }
        true
    }

    fn submit_path_action(&mut self, action: PathAction, text: &str) {
        match action {
            PathAction::Rename { source, force } => self.rename_file(&source, text, force),
            PathAction::NewFile { root } => self.create_new_path(&root, text),
            PathAction::NewDirectory { root } => {
                self.create_new_path(&root, &directory_path_text(text))
            }
        }
    }

    pub(super) fn file_ops_prompt(&self) -> Option<kawi_tui::frame::PromptMenu<'_>> {
        self.file_ops.as_ref().map(FileOpSession::prompt)
    }

    pub(super) fn handle_rename_command(
        &mut self,
        source: Option<&str>,
        name: &str,
        force: bool,
    ) -> anyhow::Result<bool> {
        let source = match source {
            Some(path) => absolute_path(path),
            None => match self.current_path() {
                Some(path) => path,
                None => {
                    self.show_message("E32: No file name".to_string());
                    return Ok(false);
                }
            },
        };
        if name.is_empty() {
            self.open_rename_prompt(source, force);
            return Ok(false);
        }
        self.rename_file(&source, name, force);
        Ok(false)
    }

    pub(super) fn handle_delete_command(
        &mut self,
        path: Option<&str>,
        force: bool,
    ) -> anyhow::Result<bool> {
        let path = match path {
            Some(path) => absolute_path(path),
            None => match self.current_path() {
                Some(path) => path,
                None => {
                    self.show_message("E32: No file name".to_string());
                    return Ok(false);
                }
            },
        };
        self.open_delete_prompt(path, force);
        Ok(false)
    }

    pub(super) fn handle_write_as_target(&mut self, name: &str, force: bool) {
        self.handle_write_target(name, force, prompt::WriteKind::WriteAs);
    }

    pub(super) fn handle_save_as_command(
        &mut self,
        name: &str,
        force: bool,
    ) -> anyhow::Result<bool> {
        self.handle_write_target(name, force, prompt::WriteKind::SaveAs);
        Ok(false)
    }

    pub(super) fn picker_new_file(&mut self, directory: PathBuf) {
        self.open_new_file_prompt(directory);
    }

    pub(super) fn picker_rename_file(&mut self, path: PathBuf) {
        self.open_rename_prompt(path, false);
    }

    pub(super) fn picker_delete_file(&mut self, path: PathBuf) {
        self.open_delete_prompt(path, false);
    }

    fn current_path(&self) -> Option<PathBuf> {
        self.editor
            .has_file()
            .then(|| absolute_path(self.editor.filename()))
    }
}

pub(super) fn current_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_default()
}

pub(super) fn absolute_path(path: &str) -> PathBuf {
    paths::absolute(path, &current_dir())
}

pub(super) fn is_directory_text(text: &str) -> bool {
    text.ends_with(['/', '\\'])
}

fn directory_path_text(text: &str) -> String {
    if is_directory_text(text) {
        text.to_string()
    } else {
        format!("{text}{}", std::path::MAIN_SEPARATOR)
    }
}

pub(super) fn path_under(path: &Path, root: &Path) -> bool {
    path == root
        || path
            .strip_prefix(root)
            .is_ok_and(|rest| !rest.as_os_str().is_empty())
}

pub(super) fn rewrite_path(path: &Path, old: &Path, new: &Path) -> Option<PathBuf> {
    if path == old {
        return Some(new.to_path_buf());
    }
    let rest = path.strip_prefix(old).ok()?;
    (!rest.as_os_str().is_empty()).then(|| new.join(rest))
}

pub(super) fn buffer_numbers_under(app: &App, target: &Path) -> Vec<usize> {
    app.editor
        .all_buffer_numbers()
        .into_iter()
        .filter(|number| {
            app.editor
                .buffer_path(*number)
                .map(|path| path_under(&absolute_path(path), target))
                .unwrap_or(false)
        })
        .collect()
}
