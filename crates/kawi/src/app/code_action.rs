use crossterm::event::KeyEvent;

use kawi_lsp::{CodeActionEntry, CodeActionEvent, CursorPosition};

use super::lsp_menu::{menu_key, MenuKey, MenuSelection};
use super::lsp_navigation::cursor;
use super::App;

pub(super) struct CodeActionSession {
    event: CodeActionEvent,
    displays: Vec<String>,
    selection: MenuSelection,
}

enum CodeActionOutcome {
    Consumed,
    Cancel,
    Apply,
}

impl CodeActionSession {
    pub(super) fn new(event: CodeActionEvent) -> Self {
        let displays = event.actions.iter().map(display_action).collect();
        Self {
            selection: MenuSelection::new(event.actions.len(), None),
            event,
            displays,
        }
    }

    pub(super) fn items(&self) -> &[String] {
        &self.displays
    }

    pub(super) fn selected(&self) -> Option<usize> {
        self.selection.selected()
    }

    fn handle_key(&mut self, key: KeyEvent) -> CodeActionOutcome {
        match menu_key(key) {
            MenuKey::Cancel => CodeActionOutcome::Cancel,
            MenuKey::Move(delta) => {
                self.selection.move_by(delta);
                CodeActionOutcome::Consumed
            }
            MenuKey::Accept => CodeActionOutcome::Apply,
            MenuKey::Other => CodeActionOutcome::Consumed,
        }
    }

    fn selected_action(&self) -> Option<&CodeActionEntry> {
        self.event.actions.get(self.selection.selected()?)
    }
}

fn display_action(action: &CodeActionEntry) -> String {
    let mut display = action.title.clone();
    if let Some(kind) = action.kind.as_deref().filter(|kind| !kind.is_empty()) {
        display.push_str(" [");
        display.push_str(kind);
        display.push(']');
    }
    if action.preferred {
        display.push_str(" (preferred)");
    }
    display
}

impl App {
    pub(super) fn request_code_actions(&mut self) -> anyhow::Result<bool> {
        let range = self.code_action_range();
        let diagnostics = self.lsp.diagnostics_in_range(range.0, range.1);
        match self.lsp.code_actions(&self.editor, range, diagnostics) {
            Ok(_) => {
                self.code_actions = None;
                self.completion = None;
            }
            Err(message) => self.show_message(message),
        }
        Ok(false)
    }

    fn code_action_range(&self) -> (CursorPosition, CursorPosition) {
        if let Some(((start_row, start_col), (end_row, end_col))) = self.editor.selection_range() {
            return (cursor(start_row, start_col), cursor(end_row, end_col));
        }
        let row = self.editor.row();
        let end = self.editor.line_len(row);
        (cursor(row, 0), cursor(row, end))
    }

    pub(super) fn handle_code_action_key(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        let Some(mut session) = self.code_actions.take() else {
            return Ok(false);
        };
        match session.handle_key(key) {
            CodeActionOutcome::Consumed => {
                self.code_actions = Some(session);
            }
            CodeActionOutcome::Cancel => {
                self.lsp.cancel_code_actions();
            }
            CodeActionOutcome::Apply => self.apply_code_action(&session),
        }
        Ok(false)
    }

    fn apply_code_action(&mut self, session: &CodeActionSession) {
        let Some(action) = session.selected_action().cloned() else {
            return;
        };
        if let Some(edit) = action.edit.as_ref() {
            if let Err(message) = self.apply_workspace_edit(edit) {
                self.show_message(message);
                return;
            }
            self.reveal_cursor();
        }
        if let Some(command) = action.command.as_ref() {
            if !self
                .lsp
                .execute_command(&session.event.path, &action.server, command)
            {
                self.show_message("lsp unavailable".to_string());
            }
        }
        self.exit_visual_if_active();
    }

    pub(super) fn handle_code_action_event(&mut self, event: CodeActionEvent) {
        if event.actions.is_empty() {
            self.code_actions = None;
            return;
        }
        self.code_actions = Some(CodeActionSession::new(event));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::Flavor;
    use crossterm::event::{KeyCode, KeyModifiers};
    use kawi_buffer::{content_hash, Buffer};
    use kawi_editor::Action;
    use kawi_lsp::{DocumentEdit, LspCommand, TextChange, WorkspaceEdit};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn event() -> CodeActionEvent {
        CodeActionEvent {
            request: 4,
            path: PathBuf::from("/tmp/action.rs"),
            actions: vec![
                CodeActionEntry {
                    server: "fake".to_string(),
                    title: "Fix import".to_string(),
                    kind: Some("quickfix".to_string()),
                    preferred: true,
                    edit: Some(WorkspaceEdit {
                        documents: vec![DocumentEdit {
                            path: PathBuf::from("/tmp/action.rs"),
                            original_hash: 0,
                            edits: vec![TextChange {
                                start: 0,
                                end: 0,
                                text: "fixed".to_string(),
                            }],
                        }],
                    }),
                    command: Some(LspCommand {
                        title: "save".to_string(),
                        command: "save".to_string(),
                        arguments: None,
                    }),
                },
                CodeActionEntry {
                    server: "fake".to_string(),
                    title: "Organize imports".to_string(),
                    kind: None,
                    preferred: false,
                    edit: None,
                    command: None,
                },
            ],
        }
    }

    #[test]
    fn displays_kind_and_preferred_state() {
        let session = CodeActionSession::new(event());
        assert_eq!(
            session.items(),
            ["Fix import [quickfix] (preferred)", "Organize imports"]
        );
    }

    #[test]
    fn navigation_wraps_and_enter_applies() {
        let mut session = CodeActionSession::new(event());
        assert!(matches!(
            session.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
            CodeActionOutcome::Consumed
        ));
        assert_eq!(session.selected(), Some(1));
        assert!(matches!(
            session.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            CodeActionOutcome::Apply
        ));
    }

    #[test]
    fn accepting_an_action_applies_its_edit() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kawi_code_action_{stamp}"));
        fs::create_dir_all(&root).expect("root");
        let path = root.join("main.rs");
        fs::write(&path, "old\n").expect("file");
        let mut app = App::new(
            Buffer::from_file(path.to_str().expect("path")).expect("buffer"),
            Flavor::Vim,
        );
        let mut event = event();
        event.path = path.clone();
        let action = &mut event.actions[0];
        action.command = None;
        let edit = action.edit.as_mut().expect("edit");
        edit.documents[0].path = path.clone();
        edit.documents[0].original_hash = content_hash(b"old\n");
        edit.documents[0].edits[0] = TextChange {
            start: 0,
            end: 3,
            text: "fixed".to_string(),
        };
        app.code_actions = Some(CodeActionSession::new(event));
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("key");
        assert!(app.code_actions.is_none());
        assert_eq!(app.editor.visible_lines(0, 1), vec!["fixed"]);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn code_action_range_uses_the_current_line_without_selection() {
        let app = App::new(Buffer::from_string("one\ntwo\n"), Flavor::Vim);
        let range = app.code_action_range();
        assert_eq!(range.0.line, 0);
        assert_eq!(range.0.column, 0);
        assert_eq!(range.1.line, 0);
        assert_eq!(range.1.column, 3);
    }

    #[test]
    fn code_action_range_uses_a_visual_selection() {
        let mut app = App::new(Buffer::from_string("one\ntwo\n"), Flavor::Vim);
        app.editor.apply(Action::EnterVisualLine);
        app.editor.apply(Action::MoveDown);
        let range = app.code_action_range();
        assert_eq!(range.0.line, 0);
        assert_eq!(range.0.column, 0);
        assert_eq!(range.1.line, 1);
        assert_eq!(range.1.column, 3);
    }

    #[test]
    fn escape_cancels() {
        let mut session = CodeActionSession::new(event());
        assert!(matches!(
            session.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            CodeActionOutcome::Cancel
        ));
    }
}
