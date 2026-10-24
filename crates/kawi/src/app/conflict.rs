use std::path::PathBuf;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use similar::TextDiff;

use super::App;

const CHOICES: [&str; 3] = ["Keep mine", "Load theirs", "Show diff"];

pub(super) struct ConflictSession {
    number: usize,
    items: Vec<String>,
    selected: usize,
}

impl ConflictSession {
    pub(super) fn new(number: usize) -> Self {
        Self {
            number,
            items: CHOICES.iter().map(|choice| (*choice).to_string()).collect(),
            selected: 0,
        }
    }

    pub(super) fn items(&self) -> &[String] {
        &self.items
    }

    pub(super) fn selected(&self) -> Option<usize> {
        Some(self.selected)
    }

    fn move_by(&mut self, delta: isize) {
        let len = self.items.len();
        self.selected = (self.selected as isize + delta).rem_euclid(len as isize) as usize;
    }
}

impl App {
    pub(super) fn handle_conflict_command(&mut self) -> Result<bool> {
        let number = self.editor.buffer_number();
        if !self.disk_conflict(number) {
            self.show_message("No disk conflict".to_string());
            return Ok(false);
        }
        self.open_conflict_prompt(number);
        Ok(false)
    }

    pub(super) fn maybe_open_conflict(&mut self) {
        if self.conflict.is_some() || self.pager.is_some() {
            return;
        }
        let number = self.editor.buffer_number();
        if !self.disk_needs_prompt(number) {
            return;
        }
        self.open_conflict_prompt(number);
    }

    fn open_conflict_prompt(&mut self, number: usize) {
        self.dismiss_lsp_menu();
        self.disk_prompt_open(number);
        let name = self
            .editor
            .buffer_name(number)
            .unwrap_or("[No Name]")
            .to_string();
        self.show_message(format!("File changed on disk: {name}"));
        self.conflict = Some(ConflictSession::new(number));
    }

    pub(super) fn handle_conflict_key(&mut self, key: KeyEvent) -> Result<bool> {
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return Ok(false);
        }
        let Some(mut session) = self.conflict.take() else {
            return Ok(false);
        };
        match key.code {
            KeyCode::Esc => {
                self.disk_dismiss_prompt(session.number);
                self.conflict = None;
                self.show_message("Conflict left unresolved; use :conflict".to_string());
            }
            KeyCode::Up | KeyCode::Char('k') => {
                session.move_by(-1);
                self.conflict = Some(session);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                session.move_by(1);
                self.conflict = Some(session);
            }
            KeyCode::Enter => self.resolve_conflict(session),
            _ => self.conflict = Some(session),
        }
        Ok(false)
    }

    fn resolve_conflict(&mut self, session: ConflictSession) {
        match session.selected {
            0 => self.keep_buffer_version(session.number),
            1 => {
                if let Err(message) = self.load_disk_version(session.number) {
                    self.show_message(message);
                    self.conflict = Some(ConflictSession::new(session.number));
                }
            }
            _ => self.show_conflict_diff(session.number),
        }
    }

    fn show_conflict_diff(&mut self, number: usize) {
        match conflict_diff(self.disk_path(number), self.editor.buffer_text(number)) {
            Ok(lines) if lines.is_empty() => {
                self.show_message("No differences".to_string());
            }
            Ok(lines) => self.show_pager(lines),
            Err(message) => self.show_message(message),
        }
    }

    fn disk_path(&self, number: usize) -> Option<PathBuf> {
        self.disk.path(number).map(PathBuf::from)
    }
}

fn conflict_diff(path: Option<PathBuf>, buffer: Option<String>) -> Result<Vec<String>, String> {
    let path = path.ok_or_else(|| "Buffer has no disk file".to_string())?;
    let disk = std::fs::read_to_string(&path)
        .map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
    let buffer = buffer.ok_or_else(|| "Buffer is not open".to_string())?;
    let diff = TextDiff::from_lines(&disk, &buffer)
        .unified_diff()
        .context_radius(3)
        .header(&format!("disk:{}", path.display()), "buffer")
        .to_string();
    if diff.trim().is_empty() {
        return Ok(Vec::new());
    }
    Ok(diff.lines().map(str::to_string).collect())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use kawi_buffer::Buffer;
    use kawi_editor::Action;

    use super::*;
    use crate::keymap::Flavor;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn edit(app: &mut App, text: &str) {
        app.editor.apply(Action::EnterInsert);
        for ch in text.chars() {
            app.editor.apply(Action::InsertChar(ch));
        }
        app.editor.apply(Action::ExitInsert);
    }

    fn conflicted(name: &str) -> (std::path::PathBuf, std::path::PathBuf, App) {
        let dir = crate::test_support::temp_dir(name);
        let path = dir.join("a.txt");
        fs::write(&path, "alpha\n").expect("write");
        let mut app = App::new(
            Buffer::from_file(path.to_str().expect("path")).expect("buffer"),
            Flavor::Vim,
        );
        app.sync_watched_files();
        edit(&mut app, "x");
        fs::write(&path, "theirs\n").expect("external");
        app.handle_disk_paths(std::slice::from_ref(&path));
        app.maybe_open_conflict();
        (dir, path, app)
    }

    #[test]
    fn active_conflicts_open_a_choice_menu() {
        let (dir, _path, mut app) = conflicted("prompt");
        assert!(app.conflict.is_some());
        assert_eq!(app.conflict.as_ref().expect("session").items().len(), 3);
        app.handle_conflict_key(key(KeyCode::Enter)).expect("keep");
        assert!(app.conflict.is_none());
        assert!(!app.disk_conflict(1));
        assert_eq!(app.editor.visible_lines(0, 1), vec!["xalpha"]);
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn choosing_load_theirs_replaces_local_changes() {
        let (dir, _path, mut app) = conflicted("load");
        app.handle_conflict_key(key(KeyCode::Down)).expect("down");
        app.handle_conflict_key(key(KeyCode::Enter)).expect("load");
        assert_eq!(app.editor.visible_lines(0, 1), vec!["theirs"]);
        assert!(!app.editor.is_dirty());
        assert!(!app.disk_conflict(1));
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn choosing_diff_shows_the_disk_and_buffer_versions() {
        let (dir, _path, mut app) = conflicted("diff");
        app.handle_conflict_key(key(KeyCode::Down)).expect("down");
        app.handle_conflict_key(key(KeyCode::Down)).expect("down");
        app.handle_conflict_key(key(KeyCode::Enter)).expect("diff");
        let lines = app.pager_lines().expect("diff pager").join("\n");
        assert!(lines.contains("disk:"));
        assert!(lines.contains("-theirs"));
        assert!(lines.contains("+xalpha"));
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn escape_dismisses_until_the_command_reopens() {
        let (dir, _path, mut app) = conflicted("dismiss");
        app.handle_conflict_key(key(KeyCode::Esc)).expect("escape");
        assert!(app.conflict.is_none());
        assert!(app.disk_conflict(1));
        app.handle_conflict_command().expect("command");
        assert!(app.conflict.is_some());
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn save_waits_for_conflict_resolution_unless_forced() {
        let (dir, path, mut app) = conflicted("save");
        app.handle_save(false).expect("save");
        assert_eq!(fs::read_to_string(&path).expect("disk"), "theirs\n");
        assert!(app.editor.is_dirty());
        app.handle_save(true).expect("force save");
        assert_eq!(fs::read_to_string(&path).expect("disk"), "xalpha\n");
        assert!(!app.disk_conflict(1));
        fs::remove_dir_all(dir).expect("cleanup");
    }
}
