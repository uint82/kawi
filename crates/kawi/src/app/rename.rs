use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_lsp::RenameReady;
use kawi_picker::TextInput;
use kawi_tui::frame::PromptMenu;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RenameStage {
    Preparing,
    Input,
}

pub(super) enum RenameOutcome {
    Consumed,
    Cancel,
    Submit { request: u64, name: String },
}

pub(super) struct RenameSession {
    request: u64,
    stage: RenameStage,
    input: TextInput,
    pristine: bool,
    label: &'static str,
}

impl RenameSession {
    pub(super) fn preparing(request: u64) -> Self {
        Self {
            request,
            stage: RenameStage::Preparing,
            input: TextInput::new(),
            pristine: false,
            label: "rename",
        }
    }

    pub(super) fn input(label: &'static str) -> Self {
        Self {
            request: 0,
            stage: RenameStage::Input,
            input: TextInput::new(),
            pristine: false,
            label,
        }
    }

    pub(super) fn request(&self) -> u64 {
        self.request
    }

    pub(super) fn set_ready(&mut self, ready: &RenameReady) -> bool {
        if ready.request != self.request || self.stage != RenameStage::Preparing {
            return false;
        }
        self.stage = RenameStage::Input;
        self.input.set_text(&ready.placeholder);
        self.pristine = true;
        true
    }

    pub(super) fn prompt(&self) -> Option<PromptMenu<'_>> {
        (self.stage == RenameStage::Input).then(|| PromptMenu {
            label: self.label,
            value: self.input.text(),
            cursor: self.input.cursor(),
        })
    }

    pub(super) fn handle_key(&mut self, key: KeyEvent) -> RenameOutcome {
        if key.code == KeyCode::Esc
            || (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c'))
        {
            return RenameOutcome::Cancel;
        }
        if self.stage == RenameStage::Preparing {
            return RenameOutcome::Consumed;
        }
        match key.code {
            KeyCode::Enter => {
                if self.input.text().is_empty() {
                    return RenameOutcome::Consumed;
                }
                return RenameOutcome::Submit {
                    request: self.request,
                    name: self.input.text().to_string(),
                };
            }
            KeyCode::Left => {
                self.pristine = false;
                self.input.move_left();
            }
            KeyCode::Right => {
                self.pristine = false;
                self.input.move_right();
            }
            KeyCode::Home => {
                self.pristine = false;
                self.input.move_home();
            }
            KeyCode::End => {
                self.pristine = false;
                self.input.move_end();
            }
            KeyCode::Backspace => {
                self.pristine = false;
                self.input.backspace();
            }
            KeyCode::Delete => {
                self.pristine = false;
                self.input.delete();
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.clear();
                self.pristine = false;
            }
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.delete_word();
                self.pristine = false;
            }
            KeyCode::Char(character)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if self.pristine {
                    self.input.clear();
                    self.pristine = false;
                }
                self.input.insert_char(character);
            }
            _ => {}
        }
        RenameOutcome::Consumed
    }

    pub(super) fn paste(&mut self, text: &str) {
        if self.stage != RenameStage::Input {
            return;
        }
        if self.pristine {
            self.input.clear();
            self.pristine = false;
        }
        self.input.insert_text(text);
    }
}

impl super::App {
    pub(super) fn request_rename(&mut self) -> anyhow::Result<bool> {
        match self.lsp.prepare_rename(&self.editor) {
            Ok(request) => self.rename = Some(RenameSession::preparing(request)),
            Err(message) => self.show_message(message),
        }
        Ok(false)
    }

    pub(super) fn handle_rename_key(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        let Some(mut session) = self.rename.take() else {
            return Ok(false);
        };
        match session.handle_key(key) {
            RenameOutcome::Cancel => self.lsp.cancel_rename(session.request()),
            RenameOutcome::Consumed => self.rename = Some(session),
            RenameOutcome::Submit { request, name } => {
                if !self.lsp.rename(request, &name) {
                    self.show_message("lsp unavailable".to_string());
                }
            }
        }
        Ok(false)
    }

    pub(super) fn paste_rename(&mut self, text: &str) -> bool {
        let Some(session) = self.rename.as_mut() else {
            return false;
        };
        session.paste(text);
        true
    }

    pub(super) fn sync_rename(&mut self) {
        if let Some(request) = self.lsp.take_rename_error() {
            if self
                .rename
                .as_ref()
                .is_some_and(|session| session.request() == request)
            {
                self.rename = None;
            }
        }
        if let Some(ready) = self.lsp.take_rename_ready() {
            if let Some(session) = self.rename.as_mut() {
                session.set_ready(&ready);
            }
        }
        if let Some(event) = self.lsp.take_rename_edit() {
            self.rename = None;
            self.handle_rename_event(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::Flavor;
    use kawi_buffer::Buffer;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn input_session() -> RenameSession {
        let mut session = RenameSession::preparing(4);
        assert!(session.set_ready(&RenameReady {
            request: 4,
            placeholder: "old_name".to_string(),
        }));
        session
    }

    #[test]
    fn typing_replaces_the_placeholder_once() {
        let mut session = input_session();
        session.handle_key(key(KeyCode::Char('n')));
        session.handle_key(key(KeyCode::Char('e')));
        session.handle_key(key(KeyCode::Char('w')));
        match session.handle_key(key(KeyCode::Enter)) {
            RenameOutcome::Submit { request, name } => {
                assert_eq!(request, 4);
                assert_eq!(name, "new");
            }
            _ => panic!("expected submit"),
        }
    }

    #[test]
    fn escape_and_control_c_cancel() {
        let mut session = input_session();
        assert!(matches!(
            session.handle_key(key(KeyCode::Esc)),
            RenameOutcome::Cancel
        ));
        assert!(matches!(
            session.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            RenameOutcome::Cancel
        ));
    }

    #[test]
    fn preparing_ignores_text_and_pristine_ready_is_empty() {
        let mut session = RenameSession::preparing(1);
        assert!(matches!(
            session.handle_key(key(KeyCode::Char('x'))),
            RenameOutcome::Consumed
        ));
        assert!(session.prompt().is_none());
        assert!(session.set_ready(&RenameReady {
            request: 1,
            placeholder: "value".to_string(),
        }));
        assert_eq!(session.prompt().map(|prompt| prompt.value), Some("value"));
        assert!(matches!(
            session.handle_key(key(KeyCode::Enter)),
            RenameOutcome::Submit { .. }
        ));
    }

    #[test]
    fn app_routes_keys_to_the_rename_prompt() {
        let mut app = super::super::App::new(Buffer::from_string("old"), Flavor::Vim);
        app.rename = Some(input_session());
        app.handle_key(key(KeyCode::Char('n'))).expect("key");
        let prompt = app
            .rename
            .as_ref()
            .and_then(RenameSession::prompt)
            .expect("prompt");
        assert_eq!(prompt.value, "n");
        app.handle_key(key(KeyCode::Esc)).expect("escape");
        assert!(app.rename.is_none());
    }

    #[test]
    fn rejects_ready_events_for_other_requests() {
        let mut session = RenameSession::preparing(2);
        assert!(!session.set_ready(&RenameReady {
            request: 3,
            placeholder: "value".to_string(),
        }));
    }
}
