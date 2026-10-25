use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::view::{GitStatusView, Item, Section};
use crate::app::App;

impl App {
    pub(crate) fn handle_git_status_key(&mut self, key: KeyEvent) -> Result<bool> {
        let Some(view) = self.git_status.view() else {
            return Ok(false);
        };
        if view.pending_discard().is_some() {
            self.handle_git_discard_confirmation(key);
            return Ok(false);
        }
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') if !has_control(key) => {
                self.git_status.close_view();
            }
            KeyCode::Char('j') | KeyCode::Down => self.move_git_status_selection(true),
            KeyCode::Char('k') | KeyCode::Up => self.move_git_status_selection(false),
            KeyCode::Char('g') | KeyCode::Home => self.move_git_status_edge(false),
            KeyCode::Char('G') | KeyCode::End => self.move_git_status_edge(true),
            KeyCode::Enter | KeyCode::Char('o') => return self.open_git_status_item(),
            KeyCode::Char('s') => self.stage_git_status_item(),
            KeyCode::Char('u') => self.unstage_git_status_item(),
            KeyCode::Char('d') => self.diff_git_status_item(),
            KeyCode::Char('x') => self.request_git_status_discard(),
            KeyCode::Char('r') => self.refresh_git_status(),
            _ => {}
        }
        Ok(false)
    }

    fn move_git_status_selection(&mut self, forward: bool) {
        if let Some(view) = self.git_status.view_mut() {
            if forward {
                view.move_next();
            } else {
                view.move_previous();
            }
        }
    }

    fn move_git_status_edge(&mut self, last: bool) {
        if let Some(view) = self.git_status.view_mut() {
            if last {
                view.move_last();
            } else {
                view.move_first();
            }
        }
    }

    fn open_git_status_item(&mut self) -> Result<bool> {
        let Some(item) = self
            .git_status
            .view()
            .and_then(GitStatusView::selected_item)
            .cloned()
        else {
            return Ok(false);
        };
        self.git_status.close_view();
        let path = self
            .git_status
            .status()
            .map(|status| status.root.join(&item.entry.path))
            .unwrap_or_else(|| item.entry.path.clone());
        self.handle_edit(Some(path.to_string_lossy().into_owned()), false)
    }

    fn stage_git_status_item(&mut self) {
        let Some(item) = self
            .git_status
            .view()
            .and_then(GitStatusView::selected_item)
            .cloned()
        else {
            return;
        };
        let Some(root) = self.git_status_root() else {
            return;
        };
        match kawi_git::stage_path(&root, &item.entry.path) {
            Ok(()) => {
                self.show_message(format!("Staged {}", item.entry.path.display()));
                self.after_git_status_change();
            }
            Err(error) => self.show_message(format!("Git stage failed: {error}")),
        }
    }

    fn unstage_git_status_item(&mut self) {
        let Some(item) = self
            .git_status
            .view()
            .and_then(GitStatusView::selected_item)
            .cloned()
        else {
            return;
        };
        let Some(root) = self.git_status_root() else {
            return;
        };
        let unborn = self
            .git_status
            .status()
            .is_some_and(kawi_git::Status::is_unborn);
        match kawi_git::unstage_path(&root, &item.entry.path, unborn) {
            Ok(()) => {
                self.show_message(format!("Unstaged {}", item.entry.path.display()));
                self.after_git_status_change();
            }
            Err(error) => self.show_message(format!("Git unstage failed: {error}")),
        }
    }

    fn diff_git_status_item(&mut self) {
        let Some(item) = self
            .git_status
            .view()
            .and_then(GitStatusView::selected_item)
            .cloned()
        else {
            return;
        };
        let Some(root) = self.git_status_root() else {
            return;
        };
        if item.section == Section::Untracked {
            self.show_untracked_diff(&item);
            return;
        }
        let staged = item.section == Section::Staged;
        match kawi_git::diff_path(&root, &item.entry.path, staged) {
            Ok(lines) if lines.is_empty() => self.show_message("No diff".to_string()),
            Ok(lines) => self.show_pager(lines),
            Err(error) => self.show_message(format!("Git diff failed: {error}")),
        }
    }

    fn show_untracked_diff(&mut self, item: &Item) {
        let path = self
            .git_status
            .status()
            .map(|status| status.root.join(&item.entry.path));
        let Some(path) = path else {
            return;
        };
        match std::fs::read_to_string(path) {
            Ok(text) => {
                let lines: Vec<String> = text.lines().map(str::to_string).collect();
                if lines.is_empty() {
                    self.show_message("Empty file".to_string());
                } else {
                    self.show_pager(lines);
                }
            }
            Err(error) => self.show_message(format!("Cannot read file: {error}")),
        }
    }

    fn request_git_status_discard(&mut self) {
        let Some(item) = self
            .git_status
            .view_mut()
            .and_then(GitStatusView::request_discard)
        else {
            self.show_message("Unstage before discarding staged changes".to_string());
            return;
        };
        let path = item.entry.path.display().to_string();
        self.show_message(format!("Discard changes to {path}? y/n"));
    }

    fn handle_git_discard_confirmation(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y' | 'Y') => self.confirm_git_status_discard(),
            KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                if let Some(view) = self.git_status.view_mut() {
                    view.cancel_discard();
                }
                self.clear_message();
            }
            _ => {}
        }
    }

    fn confirm_git_status_discard(&mut self) {
        let Some(item) = self
            .git_status
            .view_mut()
            .and_then(GitStatusView::take_discard)
        else {
            return;
        };
        let Some(root) = self.git_status_root() else {
            return;
        };
        let result = if item.section == Section::Untracked {
            kawi_git::clean_path(&root, &item.entry.path)
        } else {
            kawi_git::restore_path(&root, &item.entry.path)
        };
        match result {
            Ok(()) => {
                self.handle_disk_paths(std::slice::from_ref(&root.join(&item.entry.path)));
                self.show_message(format!("Discarded {}", item.entry.path.display()));
                self.after_git_status_change();
            }
            Err(error) => self.show_message(format!("Git discard failed: {error}")),
        }
    }

    fn after_git_status_change(&mut self) {
        let _ = self.handle_git_refresh();
        self.refresh_git_status();
    }

    fn git_status_root(&self) -> Option<std::path::PathBuf> {
        self.git_status.status().map(|status| status.root.clone())
    }
}

fn has_control(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
}

#[cfg(test)]
mod tests;
