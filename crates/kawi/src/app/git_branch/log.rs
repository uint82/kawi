use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::{Path, PathBuf};

use kawi_git::{CommitFile, LogOptions};
use kawi_picker::PickerRow;

use super::super::{picker::PickerSession, App};
use super::actions::path_relative;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogKeyOutcome {
    Unhandled,
    KeepOpen,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogAction {
    CommitFiles,
    CopySha,
    Checkout,
    CreateBranch,
    GrepAll,
    AuthorAll,
}

impl App {
    pub(crate) fn open_log_picker(
        &mut self,
        file: bool,
        grep: Option<String>,
        author: Option<String>,
    ) -> Result<bool> {
        if self.git_unavailable() {
            return Ok(false);
        }
        let root = self.git_root().unwrap_or_default();
        let path = if file {
            let Some(path) = self.editor.buffer_path(self.editor.buffer_number()) else {
                self.show_message("E32: No file name".to_string());
                return Ok(false);
            };
            let path = PathBuf::from(path);
            Some(path_relative(&root, &path).unwrap_or(path))
        } else {
            None
        };
        let options = LogOptions { path, grep, author };
        self.open_log_with_options(root, options, None)
    }

    pub(crate) fn handle_log_picker_key(
        &mut self,
        session: &PickerSession,
        key: KeyEvent,
    ) -> Result<LogKeyOutcome> {
        let Some(action) = log_action(key) else {
            return Ok(LogKeyOutcome::Unhandled);
        };
        let row = session
            .picker()
            .selected()
            .and_then(|index| session.picker().rows().get(index))
            .cloned();
        if row.is_none() && !matches!(action, LogAction::GrepAll | LogAction::AuthorAll) {
            return Ok(LogKeyOutcome::Unhandled);
        }
        let root = session.root().to_path_buf();
        let query = session.picker().query().to_string();
        let options = self.git_branch.log_options.clone().unwrap_or_default();
        let row = row.unwrap_or(PickerRow {
            display: String::new(),
            path: String::new(),
            line: None,
            column: None,
            buffer: None,
            secondary: None,
            deprecated: false,
            id: None,
        });
        if action == LogAction::CopySha {
            self.run_log_action(action, root, row, query, options)?;
            return Ok(LogKeyOutcome::KeepOpen);
        }
        self.picker = None;
        self.run_log_action(action, root, row, query, options)?;
        Ok(LogKeyOutcome::Closed)
    }

    fn run_log_action(
        &mut self,
        action: LogAction,
        root: PathBuf,
        row: PickerRow,
        query: String,
        options: LogOptions,
    ) -> Result<bool> {
        match action {
            LogAction::CommitFiles => self.open_commit_files(&root, &row.path),
            LogAction::CopySha => {
                self.editor.yank_text(&row.path);
                self.show_message(format!("Copied {}", row.path));
                Ok(false)
            }
            LogAction::Checkout => {
                self.git_branch.pending_checkout = Some(row.path.clone());
                self.show_message(format!(
                    "Checkout {} detached? [y] yes [n] cancel",
                    row.path
                ));
                Ok(false)
            }
            LogAction::CreateBranch => {
                self.open_branch_prompt(Some(row.path));
                Ok(false)
            }
            LogAction::GrepAll => self.search_all(root, options, query, true),
            LogAction::AuthorAll => self.search_all(root, options, query, false),
        }
    }

    fn search_all(
        &mut self,
        root: PathBuf,
        mut options: LogOptions,
        query: String,
        grep: bool,
    ) -> Result<bool> {
        if query.is_empty() {
            self.show_message("Type a search pattern first".to_string());
            return Ok(false);
        }
        if grep {
            options.grep = Some(query.clone());
            options.author = None;
        } else {
            options.author = Some(query.clone());
            options.grep = None;
        }
        self.open_log_with_options(root, options, Some(query))
    }

    fn open_log_with_options(
        &mut self,
        root: PathBuf,
        options: LogOptions,
        query: Option<String>,
    ) -> Result<bool> {
        match PickerSession::git_log(root, options.clone(), self.events.wake()) {
            Ok(mut session) => {
                if let Some(query) = query {
                    session.set_query(&query);
                }
                self.git_branch.log_options = Some(options);
                self.picker = Some(session);
            }
            Err(error) => self.show_message(format!("Git log failed: {error}")),
        }
        Ok(false)
    }

    pub(crate) fn open_commit_files(&mut self, root: &Path, sha: &str) -> Result<bool> {
        match kawi_git::commit_files(root, sha) {
            Ok(files) if files.is_empty() => {
                self.show_message("Commit has no file changes".to_string());
            }
            Ok(files) => {
                let rows = files.iter().map(commit_file_row).collect();
                self.git_branch.commit_sha = Some(sha.to_string());
                self.git_branch.commit_files = files;
                self.picker = Some(PickerSession::git_commit_files(root.to_path_buf(), rows));
            }
            Err(error) => self.show_message(format!("Git commit files failed: {error}")),
        }
        Ok(false)
    }

    pub(crate) fn open_commit_diff(&mut self, sha: &str) -> Result<bool> {
        let Some(root) = self.git_root() else {
            self.show_message("Not inside a git repository".to_string());
            return Ok(false);
        };
        match kawi_git::commit_patch(&root, sha) {
            Ok(lines) if lines.is_empty() => self.show_message("Commit has no diff".to_string()),
            Ok(lines) => {
                self.picker = None;
                let name = format!("git-commit-{}.diff", &sha[..sha.len().min(8)]);
                self.open_readonly_window(&name, &lines.join("\n"))?;
            }
            Err(error) => self.show_message(format!("Git commit diff failed: {error}")),
        }
        Ok(false)
    }

    pub(crate) fn open_commit_file_diff(&mut self, path: &str) -> Result<bool> {
        let Some(root) = self.git_root() else {
            self.show_message("Not inside a git repository".to_string());
            return Ok(false);
        };
        let Some(sha) = self.git_branch.commit_sha.clone() else {
            self.show_message("No commit selected".to_string());
            return Ok(false);
        };
        let Some(file) = self
            .git_branch
            .commit_files
            .iter()
            .find(|file| file.path == Path::new(path))
            .cloned()
        else {
            self.show_message("File is no longer available".to_string());
            return Ok(false);
        };
        match kawi_git::file_diff(&root, &sha, &file) {
            Ok(lines) if lines.is_empty() => self.show_message("File has no diff".to_string()),
            Ok(lines) => {
                self.picker = None;
                let name = format!("{}:{}", &sha[..sha.len().min(8)], file.path.display());
                self.open_readonly_window(&name, &lines.join("\n"))?;
            }
            Err(error) => self.show_message(format!("Git file diff failed: {error}")),
        }
        Ok(false)
    }

    pub(crate) fn handle_pending_checkout(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('y') => {}
            KeyCode::Esc | KeyCode::Char('n') => {
                self.git_branch.pending_checkout = None;
                self.clear_message();
                return Ok(false);
            }
            _ => return Ok(false),
        }
        let Some(target) = self.git_branch.pending_checkout.take() else {
            return Ok(false);
        };
        self.perform_detached(&target)
    }
}

fn log_action(key: KeyEvent) -> Option<LogAction> {
    if !key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }
    match key.code {
        KeyCode::Char('f') => Some(LogAction::CommitFiles),
        KeyCode::Char('y') => Some(LogAction::CopySha),
        KeyCode::Char('o') => Some(LogAction::Checkout),
        KeyCode::Char('b') => Some(LogAction::CreateBranch),
        KeyCode::Char('g') => Some(LogAction::GrepAll),
        KeyCode::Char('a') => Some(LogAction::AuthorAll),
        _ => None,
    }
}

fn commit_file_row(file: &CommitFile) -> PickerRow {
    let mut display = format!("{} {}", file.status, file.path.display());
    if let Some(old) = file.old_path.as_ref() {
        display.push_str(&format!(" <- {}", old.display()));
    }
    PickerRow {
        display,
        path: file.path.display().to_string(),
        line: None,
        column: None,
        buffer: None,
        secondary: None,
        deprecated: false,
        id: None,
    }
}
