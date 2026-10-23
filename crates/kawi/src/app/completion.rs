use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use kawi_lsp::{CompletionEntry, CompletionEvent, DocumentEdit, WorkspaceEdit};

use super::lsp_menu::{menu_key, MenuKey, MenuSelection};
use super::App;

pub(super) struct CompletionSession {
    event: CompletionEvent,
    displays: Vec<String>,
    selection: MenuSelection,
}

pub(super) enum CompletionOutcome {
    Consumed,
    Cancel,
    Accept,
    Fallthrough,
}

impl CompletionSession {
    pub(super) fn new(event: CompletionEvent) -> Self {
        let displays = event.items.iter().map(display_item).collect();
        let preferred = event.items.iter().position(|item| item.preselect);
        let selection = MenuSelection::new(event.items.len(), preferred);
        Self {
            event,
            displays,
            selection,
        }
    }

    pub(super) fn items(&self) -> &[String] {
        &self.displays
    }

    pub(super) fn selected(&self) -> Option<usize> {
        self.selection.selected()
    }

    pub(super) fn handle_key(&mut self, key: KeyEvent) -> CompletionOutcome {
        match menu_key(key) {
            MenuKey::Cancel => CompletionOutcome::Cancel,
            MenuKey::Move(delta) => {
                self.selection.move_by(delta);
                CompletionOutcome::Consumed
            }
            MenuKey::Accept => CompletionOutcome::Accept,
            MenuKey::Other if key.code == KeyCode::Tab => CompletionOutcome::Accept,
            MenuKey::Other => CompletionOutcome::Fallthrough,
        }
    }

    fn selected_entry(&self) -> Option<&CompletionEntry> {
        self.event.items.get(self.selection.selected()?)
    }

    pub(super) fn workspace_edit(&self) -> Option<WorkspaceEdit> {
        let entry = self.selected_entry()?;
        let mut edits = Vec::with_capacity(entry.additional.len().saturating_add(1));
        edits.push(entry.primary.clone());
        edits.extend(entry.additional.iter().cloned());
        Some(WorkspaceEdit {
            documents: vec![DocumentEdit {
                path: self.event.path.clone(),
                original_hash: self.event.original_hash,
                edits,
            }],
        })
    }
}

fn display_item(item: &CompletionEntry) -> String {
    let mut display = item.label.clone();
    if let Some(detail) = item.detail.as_deref().filter(|detail| !detail.is_empty()) {
        display.push_str("  ");
        display.push_str(detail);
    }
    if item.deprecated {
        display.push_str(" [deprecated]");
    }
    display
}

impl App {
    pub(super) fn request_completion(&mut self) -> anyhow::Result<bool> {
        self.start_completion(true);
        Ok(false)
    }

    fn start_completion(&mut self, report_errors: bool) {
        match self.lsp.completion(&self.editor) {
            Ok(_) => {
                self.completion = None;
                self.code_actions = None;
            }
            Err(message) if report_errors => self.show_message(message),
            Err(_) => {}
        }
    }

    pub(super) fn handle_completion_key(&mut self, key: KeyEvent) -> anyhow::Result<Option<bool>> {
        let Some(mut session) = self.completion.take() else {
            return Ok(None);
        };
        match session.handle_key(key) {
            CompletionOutcome::Consumed => {
                self.completion = Some(session);
                Ok(Some(false))
            }
            CompletionOutcome::Cancel => {
                self.lsp.cancel_completion();
                Ok(Some(false))
            }
            CompletionOutcome::Accept => {
                self.accept_completion(&session);
                Ok(Some(false))
            }
            CompletionOutcome::Fallthrough => {
                let refresh = self.should_refresh_completion(key);
                let result = self.handle_key(key)?;
                if refresh {
                    self.lsp.refresh(&self.editor);
                    self.start_completion(false);
                }
                Ok(Some(result))
            }
        }
    }

    fn should_refresh_completion(&self, key: KeyEvent) -> bool {
        self.editor.mode().is_insert() && is_completion_continuation_key(key)
    }

    fn accept_completion(&mut self, session: &CompletionSession) {
        let Some(entry) = session.selected_entry() else {
            return;
        };
        let Some(edit) = session.workspace_edit() else {
            return;
        };
        match self.apply_workspace_edit(&edit) {
            Ok(_) => {
                if let Some(command) = entry.command.as_ref() {
                    if !self
                        .lsp
                        .execute_command(&session.event.path, &entry.server, command)
                    {
                        self.show_message("lsp unavailable".to_string());
                    }
                }
                self.reveal_cursor();
            }
            Err(message) => self.show_message(message),
        }
    }

    pub(super) fn handle_completion_event(&mut self, event: CompletionEvent) {
        if event.items.is_empty() {
            self.completion = None;
            return;
        }
        self.completion = Some(CompletionSession::new(event));
    }
}

pub(super) fn is_completion_key(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char(' ') | KeyCode::Char('@'))
}

fn is_completion_continuation_key(key: KeyEvent) -> bool {
    if key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::ALT) {
        return false;
    }
    match key.code {
        KeyCode::Backspace => true,
        KeyCode::Char(character) => character.is_alphanumeric() || character == '_',
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::Flavor;
    use crossterm::event::KeyModifiers;
    use kawi_buffer::{content_hash, Buffer};
    use kawi_lsp::{LspCommand, TextChange};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn event() -> CompletionEvent {
        CompletionEvent {
            request: 3,
            server: "fake".to_string(),
            path: std::path::PathBuf::from("/tmp/completion.rs"),
            version: 1,
            original_hash: 0,
            items: vec![
                CompletionEntry {
                    server: "fake".to_string(),
                    label: "print".to_string(),
                    detail: Some("fn".to_string()),
                    documentation: None,
                    primary: TextChange {
                        start: 0,
                        end: 0,
                        text: "print".to_string(),
                    },
                    additional: Vec::new(),
                    deprecated: false,
                    preselect: false,
                    command: Some(LspCommand {
                        title: "save".to_string(),
                        command: "save".to_string(),
                        arguments: None,
                    }),
                },
                CompletionEntry {
                    server: "fake".to_string(),
                    label: "print_debug".to_string(),
                    detail: None,
                    documentation: None,
                    primary: TextChange {
                        start: 0,
                        end: 0,
                        text: "print_debug".to_string(),
                    },
                    additional: Vec::new(),
                    deprecated: false,
                    preselect: true,
                    command: None,
                },
            ],
        }
    }

    #[test]
    fn preselects_the_server_choice_and_wraps_navigation() {
        let mut session = CompletionSession::new(event());
        assert_eq!(session.selected(), Some(1));
        assert_eq!(session.items()[0], "print  fn");
        session.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(session.selected(), Some(0));
        session.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(session.selected(), Some(1));
    }

    #[test]
    fn enter_accepts_and_escape_cancels() {
        let mut session = CompletionSession::new(event());
        assert!(matches!(
            session.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            CompletionOutcome::Accept
        ));
        assert!(matches!(
            session.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            CompletionOutcome::Cancel
        ));
    }

    #[test]
    fn selected_item_builds_a_single_document_edit() {
        let session = CompletionSession::new(event());
        let edit = session.workspace_edit().expect("edit");
        assert_eq!(
            edit.documents[0].path,
            std::path::PathBuf::from("/tmp/completion.rs")
        );
        assert_eq!(edit.documents[0].edits.len(), 1);
        assert_eq!(edit.documents[0].edits[0].text, "print_debug");
    }

    #[test]
    fn completion_continuation_only_accepts_identifier_input() {
        for code in [KeyCode::Char('a'), KeyCode::Char('_'), KeyCode::Char('9')] {
            assert!(is_completion_continuation_key(KeyEvent::new(
                code,
                KeyModifiers::NONE
            )));
        }
        assert!(is_completion_continuation_key(KeyEvent::new(
            KeyCode::Backspace,
            KeyModifiers::NONE
        )));
        for code in [KeyCode::Char(' '), KeyCode::Char('.'), KeyCode::Enter] {
            assert!(!is_completion_continuation_key(KeyEvent::new(
                code,
                KeyModifiers::NONE
            )));
        }
        assert!(!is_completion_continuation_key(KeyEvent::new(
            KeyCode::Char('n'),
            KeyModifiers::CONTROL
        )));
    }

    #[test]
    fn accepting_a_completion_applies_the_edit() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kawi_completion_{stamp}"));
        fs::create_dir_all(&root).expect("root");
        let path = root.join("main.rs");
        fs::write(&path, "old\n").expect("file");
        let mut app = App::new(
            Buffer::from_file(path.to_str().expect("path")).expect("buffer"),
            Flavor::Vim,
        );
        let mut event = event();
        event.path = path.clone();
        event.original_hash = content_hash(b"old\n");
        event.items[0].primary = TextChange {
            start: 0,
            end: 3,
            text: "new".to_string(),
        };
        event.items[0].preselect = true;
        event.items[0].command = None;
        app.editor.apply(kawi_editor::Action::JumpTo(0, 3));
        app.completion = Some(CompletionSession::new(event));
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("key");
        assert!(app.completion.is_none());
        assert_eq!(app.editor.visible_lines(0, 1), vec!["new"]);
        assert_eq!(app.cursor_position(), (0, 2));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn typing_falls_through_to_the_editor() {
        let mut session = CompletionSession::new(event());
        assert!(matches!(
            session.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
            CompletionOutcome::Fallthrough
        ));
    }
}
