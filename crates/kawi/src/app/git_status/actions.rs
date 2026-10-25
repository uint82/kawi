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
mod tests {
    use std::fs;
    use std::process::Command;
    use std::thread;
    use std::time::{Duration, Instant};

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use kawi_buffer::Buffer;

    use crate::app::App;
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

    fn app_with_repo(name: &str) -> (std::path::PathBuf, App) {
        let dir = crate::test_support::temp_dir(name);
        git(&dir, &["init", "-q"]);
        git(&dir, &["config", "user.email", "test@example.com"]);
        git(&dir, &["config", "user.name", "Test"]);
        let path = dir.join("a.txt");
        fs::write(&path, "one\n").unwrap();
        git(&dir, &["add", "a.txt"]);
        git(&dir, &["commit", "-qm", "one"]);
        fs::write(&path, "two\n").unwrap();
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

    fn key(character: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE)
    }

    #[test]
    fn stages_unstages_and_discards_the_selected_entry() {
        let (dir, mut app) = app_with_repo("actions");
        wait_for_status(&mut app);
        app.open_git_status();
        assert!(app.git_status.has_view());
        app.handle_git_status_key(key('s')).unwrap();
        wait_for_status(&mut app);
        assert_eq!(git(&dir, &["diff", "--cached", "--name-only"]), "a.txt");
        app.handle_git_status_key(key('u')).unwrap();
        wait_for_status(&mut app);
        assert_eq!(git(&dir, &["diff", "--cached", "--name-only"]), "");
        app.handle_git_status_key(key('x')).unwrap();
        assert!(app.last_message().unwrap().contains("Discard changes"));
        app.handle_git_status_key(key('y')).unwrap();
        wait_for_status(&mut app);
        assert_eq!(fs::read_to_string(dir.join("a.txt")).unwrap(), "one\n");
        let _ = fs::remove_dir_all(dir);
    }
}
